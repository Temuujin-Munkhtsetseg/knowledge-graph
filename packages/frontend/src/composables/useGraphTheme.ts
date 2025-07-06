import { computed } from 'vue';
import { useColorMode } from '@vueuse/core';

export interface GraphTheme {
  node: {
    DirectoryNode: string;
    FileNode: string;
    DefinitionNode: string;
    default: string;
  };
  edge: string;
  edgeHover: string;
  text: string;
  background: string;
  hoverBackground: string;
  border: string;
}

export const useGraphTheme = () => {
  const colorMode = useColorMode();

  const isDark = computed(() => colorMode.value === 'dark');

  const themeColors: Record<'dark' | 'light', GraphTheme> = {
    dark: {
      node: {
        DirectoryNode: '#fbbf24', // Brighter amber for directories
        FileNode: '#34d399', // Brighter emerald for files
        DefinitionNode: '#a78bfa', // Brighter violet for definitions
        default: '#9ca3af',
      },
      edge: '#6b7280', // Much brighter gray for visibility
      edgeHover: '#f3f4f6', // Bright foreground for hover
      text: 'oklch(0.85 0 276)', // Foreground from theme
      background: 'oklch(0.24 0 0)', // Card from theme
      hoverBackground: 'oklch(0.32 0 277)', // Accent from theme
      border: 'oklch(0.48 0 0)', // Ring from theme
    },
    light: {
      node: {
        DirectoryNode: '#f59e0b', // Standard amber
        FileNode: '#10b981', // Standard emerald
        DefinitionNode: '#8b5cf6', // Standard violet
        default: '#6b7280',
      },
      edge: '#9ca3af', // Better visibility in light mode
      edgeHover: '#374151', // Darker for hover contrast
      text: 'oklch(0.145 0 0)', // Foreground from theme
      background: 'oklch(1 0 0)', // Card from theme
      hoverBackground: 'oklch(0.97 0 0)', // Accent from theme
      border: 'oklch(0.708 0 0)', // Ring from theme
    },
  };

  const currentTheme = computed(() => (isDark.value ? themeColors.dark : themeColors.light));

  const getNodeColor = (nodeType: string): string => {
    return (
      currentTheme.value.node[nodeType as keyof typeof currentTheme.value.node] ||
      currentTheme.value.node.default
    );
  };

  const getNodeSize = (nodeType: string): number => {
    switch (nodeType) {
      case 'DirectoryNode':
        return 8;
      case 'FileNode':
        return 6;
      case 'DefinitionNode':
        return 4;
      default:
        return 5;
    }
  };

  return {
    isDark,
    currentTheme,
    getNodeColor,
    getNodeSize,
    themeColors,
  };
};
