// Theme management utilities

interface ThemeData {
  kind: string;
  name: string;
}

interface ThemeChangeMessage {
  type: string;
  theme: ThemeData;
}

// Validate theme data structure
export function isValidThemeData(theme: unknown): theme is ThemeData {
  return (
    typeof theme === 'object' &&
    theme !== null &&
    typeof (theme as ThemeData).kind === 'string' &&
    typeof (theme as ThemeData).name === 'string'
  );
}

// Apply theme to document
export function applyTheme(theme: ThemeData): void {
  document.documentElement.setAttribute('data-vscode-theme-kind', theme.kind);
  document.documentElement.setAttribute('data-vscode-theme-name', theme.name);
}

// Handle theme change message
export function handleThemeChangeMessage(event: MessageEvent): void {
  // Validate origin for security
  if (!event.origin.startsWith('vscode-webview://')) {
    return;
  }

  const message = event.data as ThemeChangeMessage;

  if (message.type === 'themeChange' && isValidThemeData(message.theme)) {
    applyTheme(message.theme);
  }
}

// Set up theme change listener
export function setupThemeListener(): () => void {
  window.addEventListener('message', handleThemeChangeMessage);

  // Return cleanup function
  return () => {
    window.removeEventListener('message', handleThemeChangeMessage);
  };
}
