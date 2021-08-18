use crate::entity::file::File;
use crate::service::state::State;
use crate::service::token::Token;
use crate::util::query::Query;
use crate::{args, util};
use anyhow::{anyhow, Result};
use async_std::fs::{self, OpenOptions};
use async_std::io::prelude::WriteExt;
use serde::Deserialize;
use std::path::PathBuf;
use tide::Request;
use util::env;

#[derive(Deserialize, Debug)]
pub struct BeforeUploadRequest {
    pub filename: String,
    pub parent_id: i64,
    pub size: u64,
}

#[derive(Deserialize)]
pub struct SliceUploadRequest {
    pub index: u64,
    pub hash: String,
    pub data: Vec<u8>,
}

#[derive(Deserialize, Default, Debug)]
pub struct SliceUploadQuery {
    pub index: u64,
    pub hash: String,
}

#[derive(Deserialize)]
pub struct FinishUploadRequest {
    pub upload_id: String,
}

#[derive(Debug, Clone)]
pub struct UploadTask {
    pub filename: String,
    pub path: String,
    pub file_type: String,
    pub upload_id: String,
    pub parent_id: i64,
    pub size: u64,
    pub current_index: u64,
    pub owner_id: i64,
}

impl BeforeUploadRequest {
    pub async fn validate(&self, req: &Request<State>) -> Result<bool> {
        let token = Token::from_ext(&req)?;
        if token.permission <= 0 {
            eprintln!("User auth failed for upload request");
            return Ok(false);
        }

        if self.filename.len() == 0 || self.size <= 0 {
            eprintln!("Before upload request format error");
            return Ok(false);
        }

        let mut conn = req.state().get_pool_conn().await?;
        let folder_owner_id = File::find_file_owner(self.parent_id, &mut conn).await?;
        if folder_owner_id != token.uid {
            eprintln!("Current user and dir owner not match");
            return Ok(false);
        }

        Ok(true)
    }

    pub fn create_task(&self, upload_id: &str, owner_id: i64) -> UploadTask {
        UploadTask {
            filename: self.filename.clone(),
            size: self.size,
            upload_id: String::from(upload_id),
            current_index: 0,
            parent_id: self.parent_id,
            owner_id,
            path: String::new(),
            file_type: util::infer_file_type(&self.filename),
        }
    }
}

impl SliceUploadQuery {
    pub fn validate(&self, data: &Vec<u8>, req: &Request<State>, upload_id: &str) -> Result<bool> {
        if !validate_upload_user(&req, upload_id)? {
            eprintln!("Upload user and task not match");
            return Ok(false);
        }

        let task = req.state().find_upload_task_id(upload_id)?;
        if self.index != task.current_index {
            eprintln!("Slice index conflict");
            return Ok(false);
        }

        if !self.validate_hash(&data) {
            eprintln!("Upload data corrupted");
            return Ok(false);
        }

        Ok(true)
    }

    pub fn validate_hash(&self, data: &Vec<u8>) -> bool {
        let data_hash = md5::compute(data);

        format!("{:?}", data_hash) == self.hash
    }

    pub async fn write_tmp_file(
        &self,
        data: &Vec<u8>,
        storage: &str,
        upload_id: &str,
    ) -> Result<()> {
        let upload_tmp_dir = env::get_tmp_dir(storage).join(&upload_id);
        if !upload_tmp_dir.exists() {
            return Err(anyhow!("Upload tmp dir not exist for {}", upload_id));
        }

        let upload_tmp_file = upload_tmp_dir.join(self.index.to_string());
        let mut file = fs::File::create(upload_tmp_file).await?;
        file.write_all(data).await?;

        Ok(())
    }
}

impl FinishUploadRequest {
    pub fn validate(&self, req: &Request<State>) -> Result<bool> {
        if !validate_upload_user(req, &self.upload_id)? {
            return Ok(false);
        }

        Ok(true)
    }
}

impl UploadTask {
    pub async fn combine_slices(&mut self, storage: &str) -> Result<()> {
        let upload_tmp_dir = env::get_tmp_dir(storage).join(&self.upload_id);
        if !upload_tmp_dir.exists() {
            return Err(anyhow!("Upload tmp dir not exist for {}", self.upload_id));
        }

        let files_dir = env::get_files_dir(storage);
        if !files_dir.exists() {
            fs::create_dir_all(&files_dir).await?;
        }

        let target_filename = get_valid_filename(&files_dir, &self.filename)?;
        match target_filename
            .file_name()
            .unwrap()
            .to_owned()
            .into_string()
        {
            Ok(s) => self.path = s,
            Err(e) => {
                return Err(anyhow!("Cannot convert path to string: {:?}", e));
            }
        }
        // TODO: Check Pathbuf to string conversion on non-utf8 OS.
        let mut target_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(target_filename)
            .await?;

        for i in 0..self.current_index {
            let src_file = upload_tmp_dir.join(i.to_string());
            let src_content = fs::read(src_file).await?;
            target_file.write_all(&src_content).await?;
        }

        target_file.sync_all().await?;

        fs::remove_dir_all(upload_tmp_dir).await?;

        Ok(())
    }

    pub fn insert_file_query(&self) -> Result<Query<'_>> {
        let sql = "insert into FILE (filename, file_type, path, size, owner_id, parent_id) values (?1, ?2, ?3, ?4, ?5, ?6)";
        let query = Query::new(
            sql,
            args![
                &self.filename,
                &self.file_type,
                &self.path,
                self.size,
                self.owner_id,
                self.parent_id
            ],
        );

        Ok(query)
    }
}

fn get_valid_filename(dir: &PathBuf, filename: &str) -> Result<PathBuf> {
    let mut path = dir.join(filename);
    let mut index: u64 = 0;
    let split: Vec<&str> = filename.rsplitn(2, '.').collect();

    while path.exists() {
        let new_filename = match split.len() {
            2 => format!("{}-{}.{}", split[1], &index, split[0]),
            1 => format!("{}-{}", split[0], &index),
            _ => return Err(anyhow!("Unknown filename format: {}", filename)),
        };
        path = dir.join(new_filename);
        index += 1;
    }

    Ok(path)
}

fn validate_upload_user(req: &Request<State>, upload_id: &str) -> anyhow::Result<bool> {
    let token = Token::from_ext(req)?;
    if token.permission <= 0 {
        return Ok(false);
    }

    let task = req.state().find_upload_task_id(upload_id)?;
    if task.owner_id != token.uid {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_data() {
        let content = b"Hello world";
        println!("Content: {:?}", content);

        let data: [u8; 12] = [72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 10];
        println!("Data: {:?}", &data);

        let hash = md5::compute(&data);
        println!("hashed: {:?}", &hash);
    }

    #[test]
    fn test_hash_lib() {
        let content = b"Hello world";
        let content_hash = md5::compute(&content);

        assert_eq!(
            format!("{:?}", content_hash),
            "3e25960a79dbc69b674cd4ec67a72c62"
        );
    }

    #[test]
    fn test_validate_hash() {
        let data: Vec<u8> = b"Hello world".iter().cloned().collect();

        let slice = SliceUploadQuery {
            index: 1,
            hash: "3e25960a79dbc69b674cd4ec67a72c62".into(),
        };

        assert!(slice.validate_hash(&data));
    }
}
