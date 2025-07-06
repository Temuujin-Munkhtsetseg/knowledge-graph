<script setup lang="ts">
import { computed } from 'vue';
import { File, Folder, Code, MapPin, Hash, Type, Calendar, GitBranch } from 'lucide-vue-next';
import type { TypedGraphNode } from '@gitlab-org/gkg';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

interface Props {
  node: TypedGraphNode | null;
  x: number;
  y: number;
  visible: boolean;
}

const props = defineProps<Props>();

const nodeIcon = computed(() => {
  if (!props.node) return null;

  switch (props.node.node_type) {
    case 'DirectoryNode':
      return Folder;
    case 'FileNode':
      return File;
    case 'DefinitionNode':
      return Code;
    default:
      return null;
  }
});

const nodeColor = computed(() => {
  if (!props.node) return 'bg-muted';

  switch (props.node.node_type) {
    case 'DirectoryNode':
      return 'bg-amber-500';
    case 'FileNode':
      return 'bg-emerald-500';
    case 'DefinitionNode':
      return 'bg-violet-500';
    default:
      return 'bg-muted';
  }
});

const formatLocation = (lineNumber: number, startByte: bigint, endByte: bigint) => {
  const start = Number(startByte);
  const end = Number(endByte);
  return `Line ${lineNumber}, ${start}-${end}`;
};

const getFileExtension = (path: string) => {
  const lastDot = path.lastIndexOf('.');
  return lastDot > 0 ? path.substring(lastDot) : '';
};

const tooltipPosition = computed(() => {
  const graphContainer = document.querySelector('.graph-container');
  if (!graphContainer) {
    return { top: '1rem', right: '1rem' };
  }

  const containerRect = graphContainer.getBoundingClientRect();
  return {
    top: `${containerRect.top + 16}px`,
    right: `${window.innerWidth - containerRect.right + 16}px`,
  };
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible && node"
      class="fixed z-50 pointer-events-none transition-all duration-200 ease-out"
      :style="{
        top: tooltipPosition.top,
        right: tooltipPosition.right,
      }"
    >
      <Card class="w-72 sm:w-80 shadow-lg border-2 bg-background/95 backdrop-blur-sm">
        <div class="flex flex-col p-3 space-y-2">
          <div class="flex items-start gap-2">
            <div
              class="flex items-center justify-center w-4 h-4 sm:w-5 sm:h-5 rounded-full flex-shrink-0 mt-0.5"
              :class="nodeColor"
            >
              <component :is="nodeIcon" class="w-2 h-2 sm:w-2.5 sm:h-2.5 text-white" />
            </div>
            <span
              class="break-words whitespace-normal min-w-0 flex-1 text-xs sm:text-sm font-medium leading-tight"
              >{{ node.label }}</span
            >
            <Badge variant="outline" class="text-xs flex-shrink-0 px-1.5 py-0.5 mt-0.5">
              {{ node.node_type.replace('Node', '') }}
            </Badge>
          </div>

          <!-- Directory Node Info -->
          <div v-if="node.node_type === 'DirectoryNode'" class="space-y-2">
            <div class="flex items-start gap-2 text-xs sm:text-sm text-muted-foreground">
              <MapPin class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-words whitespace-normal min-w-0">{{ node.properties.path }}</span>
            </div>
            <div
              v-if="node.properties.absolute_path"
              class="flex items-start gap-2 text-xs sm:text-sm text-muted-foreground"
            >
              <Folder class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-words whitespace-normal min-w-0">{{
                node.properties.absolute_path
              }}</span>
            </div>
            <div
              v-if="node.properties.repository_name"
              class="flex items-start gap-2 text-xs sm:text-sm text-muted-foreground"
            >
              <GitBranch class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-words whitespace-normal min-w-0">{{
                node.properties.repository_name
              }}</span>
            </div>
          </div>

          <!-- File Node Info -->
          <div v-if="node.node_type === 'FileNode'" class="space-y-2">
            <div class="flex items-start gap-2 text-xs sm:text-sm text-muted-foreground">
              <MapPin class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-words whitespace-normal min-w-0">{{ node.properties.path }}</span>
            </div>
            <div class="flex items-center gap-2 text-xs sm:text-sm">
              <Type class="w-3 h-3 sm:w-4 sm:h-4 flex-shrink-0" />
              <Badge variant="secondary" class="text-xs">
                {{ node.properties.language || 'Unknown' }}
              </Badge>
              <Badge variant="outline" class="text-xs">
                {{ node.properties.extension || getFileExtension(node.properties.path) }}
              </Badge>
            </div>
            <div
              v-if="node.properties.absolute_path"
              class="flex items-start gap-2 text-xs sm:text-sm text-muted-foreground"
            >
              <File class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-words whitespace-normal min-w-0">{{
                node.properties.absolute_path
              }}</span>
            </div>
            <div
              v-if="node.properties.repository_name"
              class="flex items-start gap-2 text-xs sm:text-sm text-muted-foreground"
            >
              <GitBranch class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-words whitespace-normal min-w-0">{{
                node.properties.repository_name
              }}</span>
            </div>
          </div>

          <!-- Definition Node Info -->
          <div v-if="node.node_type === 'DefinitionNode'" class="space-y-2">
            <div class="flex items-start gap-2 text-xs sm:text-sm text-muted-foreground">
              <MapPin class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-words whitespace-normal min-w-0">{{ node.properties.path }}</span>
            </div>
            <div v-if="node.properties.fqn" class="flex items-start gap-2 text-xs sm:text-sm">
              <Hash class="w-3 h-3 sm:w-4 sm:h-4 mt-0.5 flex-shrink-0" />
              <span class="break-all whitespace-normal font-mono text-xs min-w-0">{{
                node.properties.fqn
              }}</span>
            </div>
            <div class="flex items-center gap-2 text-xs sm:text-sm">
              <Type class="w-3 h-3 sm:w-4 sm:h-4 flex-shrink-0" />
              <Badge variant="secondary" class="text-xs">
                {{ node.properties.definition_type }}
              </Badge>
              <Badge variant="outline" class="text-xs">
                {{ node.properties.total_locations }} location{{
                  node.properties.total_locations !== 1 ? 's' : ''
                }}
              </Badge>
            </div>
            <Separator />
            <div class="text-xs text-muted-foreground">
              <div class="flex items-center gap-2">
                <Calendar class="w-3 h-3 flex-shrink-0" />
                <span class="min-w-0">{{
                  formatLocation(
                    node.properties.primary_line_number,
                    node.properties.primary_start_byte,
                    node.properties.primary_end_byte,
                  )
                }}</span>
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  </Teleport>
</template>

<style scoped>
.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
