<script setup lang="ts">
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface Props {
  path: string;
}

defineProps<Props>();

const getShortPath = (path: string) => {
  if (!path) return '';
  // Handle both Windows and Unix path separators
  const separator = path.includes('\\') ? '\\' : '/';
  const parts = path.split(separator);

  if (parts.length <= 2) return path;

  // Show last 2 parts with proper separator
  const shortPath = parts.slice(-2).join(separator);
  return `${separator}${shortPath}`;
};
</script>

<template>
  <TooltipProvider>
    <Tooltip>
      <TooltipTrigger as-child>
        <div
          class="flex items-center gap-1.5 p-1 rounded hover:bg-muted/40 transition-colors cursor-default"
          :title="path"
        >
          <div class="w-1.5 h-1.5 rounded-full bg-muted-foreground/40 flex-shrink-0" />
          <span class="font-mono text-xs truncate text-muted-foreground/80">
            {{ getShortPath(path) }}
          </span>
        </div>
      </TooltipTrigger>
      <TooltipContent side="bottom" class="max-w-md">
        <p class="font-mono text-xs break-all">
          {{ path }}
        </p>
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>
