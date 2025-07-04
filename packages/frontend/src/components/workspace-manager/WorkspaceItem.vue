<script setup lang="ts">
import { Trash2, MoreHorizontal } from 'lucide-vue-next';
import type { TSWorkspaceFolderInfo } from '@gitlab-org/gkg';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useDeleteWorkspace } from '@/hooks/api';
import { Dialog, DialogContent, DialogTrigger } from '@/components/ui/dialog';

interface Props {
  workspace: TSWorkspaceFolderInfo;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  refresh: [];
}>();

const { mutate: deleteWorkspace, isPending: isDeleting } = useDeleteWorkspace();

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

const formatPath = (path: string) => {
  const parts = path.split('/');
  return parts[parts.length - 1] || path;
};

const formatDate = (dateString: string | null) => {
  if (!dateString) return '';
  return new Date(dateString).toLocaleDateString();
};

const handleDelete = () => {
  if (!props.workspace?.workspace_folder_path) return;

  deleteWorkspace(
    { workspace_folder_path: props.workspace.workspace_folder_path },
    {
      onSuccess: () => {
        emit('refresh');
      },
    },
  );
};
</script>

<template>
  <div class="group relative">
    <div class="border border-border bg-card hover:bg-muted/30 transition-colors rounded-sm">
      <div class="flex items-start justify-between gap-2 p-2">
        <div class="flex-1 min-w-0 space-y-1.5">
          <slot name="trigger">
            <!-- Workspace Name -->
            <div class="text-xs font-medium text-foreground truncate">
              {{ formatPath(workspace?.workspace_folder_path || 'Unknown workspace') }}
            </div>

            <!-- Status and Date in aligned layout -->
            <div class="flex items-center gap-2">
              <Badge
                :variant="getStatusVariant(workspace?.status || 'unknown')"
                class="text-xs h-3 px-1"
              >
                {{ workspace?.status || 'unknown' }}
              </Badge>
              <span class="text-xs text-muted-foreground font-mono">
                {{ formatDate(workspace?.last_indexed_at || null) }}
              </span>
            </div>

            <!-- Path below -->
            <div
              class="text-xs text-muted-foreground/70 truncate font-mono"
              :title="workspace?.workspace_folder_path || 'Unknown path'"
            >
              {{ workspace?.workspace_folder_path || 'Unknown path' }}
            </div>
          </slot>
        </div>

        <!-- Actions Menu -->
        <div
          class="flex items-center gap-1 flex-shrink-0 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity"
        >
          <Badge
            v-if="(workspace?.project_count || 0) > 1"
            variant="secondary"
            class="text-xs h-3 px-1 bg-muted/60"
          >
            {{ workspace?.project_count || 0 }}
          </Badge>
          <Dialog>
            <DialogTrigger as-child>
              <Button
                variant="ghost"
                size="sm"
                class="h-5 w-5 p-0 hover:bg-muted/60"
                :aria-label="`Actions for ${formatPath(workspace?.workspace_folder_path || 'Unknown workspace')}`"
              >
                <MoreHorizontal class="h-3 w-3" />
              </Button>
            </DialogTrigger>
            <DialogContent class="max-w-sm border-border">
              <div class="space-y-3">
                <div class="space-y-1">
                  <h3 class="text-xs font-medium text-foreground">Workspace Actions</h3>
                  <p class="text-xs text-muted-foreground truncate font-mono">
                    {{ workspace?.workspace_folder_path || 'Unknown path' }}
                  </p>
                </div>
                <Button
                  :disabled="isDeleting"
                  variant="destructive"
                  size="sm"
                  class="w-full justify-start h-6 text-xs"
                  @click="handleDelete"
                >
                  <Trash2 class="h-3 w-3 mr-1.5" />
                  {{ isDeleting ? 'Deleting...' : 'Delete Workspace' }}
                </Button>
              </div>
            </DialogContent>
          </Dialog>
        </div>
      </div>
    </div>
  </div>
</template>
