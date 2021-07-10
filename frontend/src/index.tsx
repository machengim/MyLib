import ReactDOM from 'react-dom';
import { Router } from 'react-router';
import { Switch, Route } from 'react-router-dom';
import { createBrowserHistory } from 'history';
import Home from './pages/Home';
import Login from './pages/Login';
import Setup from './pages/Setup';
import Error404 from './pages/Error404';
import './index.css';

const history = createBrowserHistory();

ReactDOM.render(
  <Router history={history}>
    <App />
  </Router>,
  document.getElementById('root')
);

function App() {
  const path = window.location.pathname;

  const renderPage = (path: string) => {
    switch (path) {
      case '/':
        return <Home />;
      case '/setup':
        return <Setup />;
      case '/login':
        return <Login />;
      default:
        return <Error404 />;
    }
  };

  return (
    <>
      {renderPage(path)}

      <Switch>
        <Route path="/about">
          <About />
        </Route>
        <Route path="/users">
          <Users />
        </Route>
      </Switch>
    </>
  );
}

function About() {
  return <h2>About</h2>;
}

function Users() {
  return <h2>Users</h2>;
}
