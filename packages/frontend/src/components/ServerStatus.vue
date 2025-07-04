<script setup lang="ts">
import { computed } from 'vue';
import { useServerInfo } from '@/hooks/api';
import { Button } from '@/components/ui/button';

const {
  data: serverInfo,
  isLoading: serverLoading,
  error: serverError,
  refetch: refetchServer,
} = useServerInfo();

const isLoading = computed(() => serverLoading.value);
const hasError = computed(() => serverError.value);

const statusColorAndText = computed(() => {
  if (hasError.value) return { color: 'text-red-600', text: 'Error' };
  if (isLoading.value) return { color: 'text-gray-600', text: 'Loading' };
  if (serverInfo.value) return { color: 'text-green-600', text: 'Healthy' };
  return { color: 'text-gray-600', text: 'Unknown' };
});

const handleRefresh = () => {
  refetchServer();
};
</script>

<template>
  <div class="p-4 border rounded-lg space-y-3">
    <div class="flex items-center justify-between">
      <h2 class="text-lg font-semibold">Server Status</h2>
      <Button :disabled="isLoading" size="sm" variant="outline" @click="handleRefresh">
        Refresh
      </Button>
    </div>

    <div v-if="isLoading" class="text-sm text-gray-600">Loading server information...</div>

    <div v-else-if="hasError" class="text-sm text-red-600">Failed to load server information</div>

    <div v-else class="space-y-2">
      <div class="flex justify-between text-sm">
        <span class="font-medium">Port:</span>
        <span>{{ serverInfo?.port }}</span>
      </div>

      <div class="flex justify-between text-sm">
        <span class="font-medium">Health:</span>
        <span :class="statusColorAndText.color">
          {{ statusColorAndText.text }}
        </span>
      </div>
    </div>
  </div>
</template>
