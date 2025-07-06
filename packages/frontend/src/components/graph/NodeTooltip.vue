<script setup lang="ts">
import { computed } from 'vue';
import { File, Folder, Code, MapPin, Hash, Type, Calendar, GitBranch } from 'lucide-vue-next';
import type { TypedGraphNode } from '@gitlab-org/gkg';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
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

const shouldFlipTooltip = computed(() => {
  return props.y > 400; // Simple threshold instead of window height
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible && node"
      class="fixed z-50 pointer-events-none"
      :style="{
        left: `${x + 10}px`,
        top: `${y - 10}px`,
        transform: shouldFlipTooltip ? 'translateY(-100%)' : 'translateY(0)',
      }"
    >
      <Card class="w-80 shadow-lg border-2 bg-background/95 backdrop-blur-sm">
        <CardHeader class="pb-3">
          <CardTitle class="flex items-center gap-2 text-base">
            <div class="flex items-center justify-center w-6 h-6 rounded-full" :class="nodeColor">
              <component :is="nodeIcon" class="w-3 h-3 text-white" />
            </div>
            <span class="truncate">{{ node.label }}</span>
            <Badge variant="outline" class="ml-auto text-xs">
              {{ node.node_type.replace('Node', '') }}
            </Badge>
          </CardTitle>
        </CardHeader>

        <CardContent class="pt-0 space-y-3">
          <!-- Directory Node Info -->
          <div v-if="node.node_type === 'DirectoryNode'" class="space-y-2">
            <div class="flex items-center gap-2 text-sm text-muted-foreground">
              <MapPin class="w-4 h-4" />
              <span class="truncate">{{ node.properties.path }}</span>
            </div>
            <div
              v-if="node.properties.absolute_path"
              class="flex items-center gap-2 text-sm text-muted-foreground"
            >
              <Folder class="w-4 h-4" />
              <span class="truncate">{{ node.properties.absolute_path }}</span>
            </div>
            <div
              v-if="node.properties.repository_name"
              class="flex items-center gap-2 text-sm text-muted-foreground"
            >
              <GitBranch class="w-4 h-4" />
              <span class="truncate">{{ node.properties.repository_name }}</span>
            </div>
          </div>

          <!-- File Node Info -->
          <div v-if="node.node_type === 'FileNode'" class="space-y-2">
            <div class="flex items-center gap-2 text-sm text-muted-foreground">
              <MapPin class="w-4 h-4" />
              <span class="truncate">{{ node.properties.path }}</span>
            </div>
            <div class="flex items-center gap-2 text-sm">
              <Type class="w-4 h-4" />
              <Badge variant="secondary" class="text-xs">
                {{ node.properties.language || 'Unknown' }}
              </Badge>
              <Badge variant="outline" class="text-xs">
                {{ node.properties.extension || getFileExtension(node.properties.path) }}
              </Badge>
            </div>
            <div
              v-if="node.properties.absolute_path"
              class="flex items-center gap-2 text-sm text-muted-foreground"
            >
              <File class="w-4 h-4" />
              <span class="truncate">{{ node.properties.absolute_path }}</span>
            </div>
            <div
              v-if="node.properties.repository_name"
              class="flex items-center gap-2 text-sm text-muted-foreground"
            >
              <GitBranch class="w-4 h-4" />
              <span class="truncate">{{ node.properties.repository_name }}</span>
            </div>
          </div>

          <!-- Definition Node Info -->
          <div v-if="node.node_type === 'DefinitionNode'" class="space-y-2">
            <div class="flex items-center gap-2 text-sm text-muted-foreground">
              <MapPin class="w-4 h-4" />
              <span class="truncate">{{ node.properties.path }}</span>
            </div>
            <div v-if="node.properties.fqn" class="flex items-center gap-2 text-sm">
              <Hash class="w-4 h-4" />
              <span class="truncate font-mono text-xs">{{ node.properties.fqn }}</span>
            </div>
            <div class="flex items-center gap-2 text-sm">
              <Type class="w-4 h-4" />
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
            <div class="text-xs text-muted-foreground space-y-1">
              <div class="flex items-center gap-2">
                <Calendar class="w-3 h-3" />
                <span>{{
                  formatLocation(
                    node.properties.primary_line_number,
                    node.properties.primary_start_byte,
                    node.properties.primary_end_byte,
                  )
                }}</span>
              </div>
            </div>
          </div>
        </CardContent>
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
