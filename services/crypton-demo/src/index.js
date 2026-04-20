import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App';
import reportWebVitals from './reportWebVitals';

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// Remove pre-render skeleton once React has painted its first frame
requestAnimationFrame(() => {
  requestAnimationFrame(() => {
    const sk = document.getElementById('pre-render-skeleton');
    if (sk) sk.remove();
  });
});

reportWebVitals();
