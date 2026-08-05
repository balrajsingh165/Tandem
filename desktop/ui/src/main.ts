/**
 * Front-end entry: instantiates App.svelte, initializes the IPC client
 * connection to the daemon, and installs global error reporting.
 */

import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

window.addEventListener('error', (event) => {
  console.error('tandem-ui: uncaught error', event.error ?? event.message);
});

window.addEventListener('unhandledrejection', (event) => {
  console.error('tandem-ui: unhandled rejection', event.reason);
});

const target = document.getElementById('app');
if (!target) {
  throw new Error('tandem-ui: #app mount point missing from index.html');
}

export default mount(App, { target });
