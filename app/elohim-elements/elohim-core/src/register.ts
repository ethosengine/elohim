import { ElohimButton } from './elohim-button.js';

if (!customElements.get('elohim-button')) {
  customElements.define('elohim-button', ElohimButton);
}
