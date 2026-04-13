/**
 * Entry point for Voyage DL React application
 */

import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

// Get or create root element
const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('Root element not found');
}

// Render app
ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
