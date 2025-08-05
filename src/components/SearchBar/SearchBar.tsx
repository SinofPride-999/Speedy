import React, { useState, useRef, useEffect } from 'react';
import { useKeyboardShortcut } from './useKeyboardShortcut';
import { invoke } from '@tauri-apps/api/core';
import './SearchBar.css';
import LightningIcon from './LightningIcon';

type SearchResult = {
  path: string;
  name: string;
  type: 'file' | 'folder' | 'app';
  score?: number;
};

const SearchBar: React.FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);
  const debounceTimer = useRef<number | null>(null);

  const toggleVisibility = () => {
    setIsVisible(!isVisible);
    setSearchQuery('');
    setResults([]);
    setSelectedIndex(-1);
  };

  useKeyboardShortcut(' ', true, false, false, toggleVisibility);

  useEffect(() => {
    if (isVisible && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [isVisible]);

  const performSearch = async (query: string) => {
    if (query.length < 2) return [];
    
    try {
        // Call the unified search command instead of separate ones
        const results = await invoke<SearchResult[]>('search', { query });
        return results;
    } catch (error) {
        console.error('Search error:', error);
        return [];
    }
  };

  useEffect(() => {
    if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
    }

    if (searchQuery.trim() === '') {
        setResults([]);
        setSelectedIndex(-1);
        return;
    }

    debounceTimer.current = setTimeout(async () => {
        setIsSearching(true);
        try {
            const searchResults = await performSearch(searchQuery);
            setResults(searchResults);
        } catch (error) {
            console.error('Search failed:', error);
            setResults([]);
        } finally {
            setIsSearching(false);
        }
    }, 200);

    return () => {
        if (debounceTimer.current) {
            clearTimeout(debounceTimer.current);
        }
    };
  }, [searchQuery]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!isVisible) return;

      if (event.key === 'Escape') {
        setIsVisible(false);
      } else if (event.key === 'ArrowDown') {
        event.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, results.length - 1));
      } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, -1));
      } else if (event.key === 'Enter' && selectedIndex >= 0 && results[selectedIndex]) {
        handleResultClick(results[selectedIndex]);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isVisible, results, selectedIndex]);

  useEffect(() => {
    if (selectedIndex >= 0 && resultsRef.current) {
      const selectedItem = resultsRef.current.children[selectedIndex] as HTMLElement;
      if (selectedItem) {
        selectedItem.scrollIntoView({ block: 'nearest' });
      }
    }
  }, [selectedIndex]);

  const handleResultClick = async (result: SearchResult) => {
    try {
        if (result.type === 'app') {
            // For applications, use launch_app
            await invoke('launch_app', { path: result.path });
        } else {
            // For files/folders, use open_path
            await invoke('open_path', { path: result.path });
        }
        setIsVisible(false);
    } catch (error) {
        console.error('Failed to open:', error);
        // Show error to user
        alert(`Failed to open: ${error}`);
    }
  };

  return (
    <>      
      <div className={`search-container ${isVisible ? 'visible' : ''}`}>
        <div className="search-input-container">
          <svg className="search-icon" viewBox="0 0 24 24">
            <path d="M15.5 14h-.79l-.28-.27a6.5 6.5 0 0 0 1.48-5.34c-.47-2.78-2.79-5-5.59-5.34a6.505 6.505 0 0 0-7.27 7.27c.34 2.8 2.56 5.12 5.34 5.59a6.5 6.5 0 0 0 5.34-1.48l.27.28v.79l4.25 4.25c.41.41 1.08.41 1.49 0 .41-.41.41-1.08 0-1.49L15.5 14zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z" />
          </svg>
          <input
            ref={searchInputRef}
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Speedy’s on it ... Just type! ⚡"
            className="search-input"
            aria-label="Search input"
          />
          {isSearching ? (
            <div className="search-spinner" aria-label="Searching..." />
          ) : (
            results.length > 0 && (
              <div className="result-count">
                {results.length} {results.length === 1 ? 'result' : 'results'}
              </div>
            )
          )}
        </div>
        
        {results.length > 0 && (
          <div 
            className="search-results" 
            ref={resultsRef}
            role="listbox"
            aria-label="Search results"
          >
            {results.map((result, index) => (
              <div 
                key={`${result.path}-${index}`}
                className={`search-result-item ${selectedIndex === index ? 'selected' : ''}`}
                onClick={() => handleResultClick(result)}
                onDoubleClick={() => handleResultClick(result)}
                data-type={result.type}
                style={{ '--index': index } as React.CSSProperties}
                role="option"
                aria-selected={selectedIndex === index}
              >
                <div className="result-icon" aria-hidden="true">
                  {getIconForType(result.type)}
                </div>
                <div className="result-details">
                  <div className="result-title">
                    {result.name}
                    {result.score && (
                      <span className="result-score">{Math.round(result.score * 100)}%</span>
                    )}
                  </div>
                  <div className="result-path">{result.path}</div>
                </div>
                {selectedIndex === index && (
                  <div className="enter-hint" aria-hidden="true">
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {!isSearching && results.length === 0 && searchQuery.length >= 2 && (
          <div className="search-result-item">
            No results found for "{searchQuery}"
          </div>
        )}
      </div>
    </>
  );
};

const getIconForType = (type: 'file' | 'folder' | 'app') => {
  switch (type) {
    case 'file':
      return (
        <svg viewBox="0 0 24 24">
          <path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zM6 20V4h7v5h5v11H6z" />
        </svg>
      );
    case 'folder':
      return (
        <svg viewBox="0 0 24 24">
          <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z" />
        </svg>
      );
    case 'app':
      return (
        <svg viewBox="0 0 24 24">
          <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM12 8c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0 10c-2.2 0-4-1.8-4-4s1.8-4 4-4 4 1.8 4 4-1.8 4-4 4z" />
        </svg>
      );
  }
};

export default SearchBar;