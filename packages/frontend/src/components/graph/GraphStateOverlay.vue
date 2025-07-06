<script setup lang="ts">
import { Loader2, Network } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';

interface Props {
  isLoading: boolean;
  error: Error | null;
  hasData: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  (e: 'refresh'): void;
}>();
</script>

<template>
  <div
    v-if="isLoading || error || !hasData"
    class="absolute inset-0 flex items-center justify-center bg-background/50"
  >
    <div v-if="isLoading" class="flex items-center gap-2 text-muted-foreground">
      <Loader2 class="h-4 w-4 animate-spin" />
      <span>Loading graph...</span>
    </div>
    <div v-else-if="error" class="text-center p-4">
      <p class="text-destructive mb-2">Failed to load graph data</p>
      <p class="text-sm text-muted-foreground mb-4">{{ error.message }}</p>
      <Button variant="outline" size="sm" @click="emit('refresh')">Try Again</Button>
    </div>
    <div v-else-if="!hasData" class="text-center p-4">
      <Network class="h-10 w-10 text-muted-foreground mx-auto mb-3" />
      <p class="text-muted-foreground mb-2">No graph data available</p>
      <p class="text-sm text-muted-foreground">
        Project may not be indexed or contains no analyzable files.
      </p>
    </div>
  </div>
</template>
