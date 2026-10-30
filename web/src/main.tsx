import {createRoot} from 'react-dom/client';
import App from './App.tsx';
import { PairingGate } from './components/common/PairingGate';
import './index.css';
import { initSavedTheme } from './utils/themeEngine';
import { initSavedFonts } from './utils/fontEngine';

// Initialize custom or VS Code theme and typography if saved in user preferences
initSavedTheme();
initSavedFonts();

createRoot(document.getElementById('root')!).render(
  <PairingGate>
    <App />
  </PairingGate>
);
