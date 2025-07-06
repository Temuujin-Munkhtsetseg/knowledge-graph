<script setup lang="ts">
import { ZoomIn, ZoomOut, RotateCcw, Maximize2, Loader2 } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';

interface Props {
  isLoading: boolean;
  hasData: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  (e: 'zoom-in'): void;
  (e: 'zoom-out'): void;
  (e: 'reset-view'): void;
  (e: 'toggle-fullscreen'): void;
  (e: 'refresh'): void;
}>();
</script>

<template>
  <div class="flex items-center gap-2">
    <div v-if="hasData" class="flex items-center gap-1 border rounded-md p-1">
      <Button variant="ghost" size="sm" class="h-6 w-6 p-0" @click="emit('zoom-in')">
        <ZoomIn class="h-3 w-3" />
      </Button>
      <Button variant="ghost" size="sm" class="h-6 w-6 p-0" @click="emit('zoom-out')">
        <ZoomOut class="h-3 w-3" />
      </Button>
      <Button variant="ghost" size="sm" class="h-6 w-6 p-0" @click="emit('reset-view')">
        <RotateCcw class="h-3 w-3" />
      </Button>
      <Button variant="ghost" size="sm" class="h-6 w-6 p-0" @click="emit('toggle-fullscreen')">
        <Maximize2 class="h-3 w-3" />
      </Button>
    </div>
    <Button variant="outline" size="sm" :disabled="isLoading" @click="emit('refresh')">
      <Loader2 v-if="isLoading" class="h-3 w-3 animate-spin mr-1" />
      Refresh
    </Button>
  </div>
</template>
