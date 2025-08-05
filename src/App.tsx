import React from 'react';
import SearchBar from './components/SearchBar/SearchBar';
import './App.css';

const App: React.FC = () => {
  return (
    <>
      <main>
        <div className="stuffs">
          <SearchBar />
          <div className="shortcut-hint">
          </div>
        </div>
      </main>
    </>
  );
};

export default App;