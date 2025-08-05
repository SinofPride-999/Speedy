import React from 'react';
import SearchBar from './components/SearchBar/SearchBar';
import './App.css';

const App: React.FC = () => {
  return (
    <>
      <SearchBar />
      <div className="shortcut-hint">
        Press <kbd>⌘</kbd> + <kbd>Space</kbd> on Mac or <kbd>Ctrl</kbd> + <kbd>Space</kbd> on Windows
      </div>
    </>
  );
};

export default App;