<script setup lang="ts">
import { computed } from 'vue';
import { ChevronRight, AlertCircle, Database, FolderOpen, type LucideIcon } from 'lucide-vue-next';
import StyledPath from '@/components/common/StyledPath.vue';
import { Badge } from '@/components/ui/badge';

interface Props {
  name: string;
  status: string;
  lastIndexedAt: string | null;
  path: string;
  isCollapsible: boolean;
  isOpen?: boolean;
}

const props = defineProps<Props>();

const getStatusVariant = (status: string) => {
  switch (status.toLowerCase()) {
    case 'indexed':
      return 'default';
    case 'indexing':
      return 'secondary';
    case 'error':
      return 'destructive';
    default:
      return 'outline';
  }
};

const formatDate = (dateString: string | null) => {
  if (!dateString) return '';
  const date = new Date(dateString);
  return `${date.toLocaleDateString()} ${date.toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  })}`;
};

const getStatusIcon = (status: string): LucideIcon => {
  switch (status.toLowerCase()) {
    case 'error':
      return AlertCircle;
    case 'indexed':
      return Database;
    default:
      return FolderOpen;
  }
};

const statusIcon = computed(() => getStatusIcon(props.status));
</script>

<template>
  <div class="space-y-1.5 text-left">
    <!-- Title row with VS Code styling -->
    <div class="flex items-center gap-1.5">
      <ChevronRight
        v-if="isCollapsible"
        class="h-3 w-3 transition-transform duration-200 text-muted-foreground flex-shrink-0"
        :class="{ 'rotate-90': isOpen }"
      />
      <component :is="statusIcon" v-else class="h-3 w-3 flex-shrink-0 text-muted-foreground" />

      <span class="text-xs font-medium text-foreground truncate flex-1">
        {{ name }}
      </span>
    </div>

    <!-- Details row with VS Code styling -->
    <div class="pl-4 space-y-1">
      <!-- Status and date -->
      <div class="flex items-center gap-2">
        <Badge :variant="getStatusVariant(status)" class="text-xs h-3 px-1">
          {{ status }}
        </Badge>
        <span class="text-xs text-muted-foreground font-mono">
          {{ formatDate(lastIndexedAt) }}
        </span>
      </div>
      <!-- Path -->
      <StyledPath :path="path" />
    </div>
  </div>
</template>
