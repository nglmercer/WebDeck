import { mount } from 'svelte';
import App from './App.svelte';
import './styles/tokens.css';
import './styles/base.css';
import './features/deck/deck.css';
mount(App, { target: document.getElementById('app')! });
