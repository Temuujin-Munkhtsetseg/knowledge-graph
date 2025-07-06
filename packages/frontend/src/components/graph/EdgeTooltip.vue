<script setup lang="ts">
import { computed } from 'vue';
import { ArrowRight, Folder, File, Code, Link } from 'lucide-vue-next';
import type { TypedGraphNode, GraphRelationship } from '@gitlab-org/gkg';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

interface Props {
  relationship: GraphRelationship | null;
  sourceNode: TypedGraphNode | null;
  targetNode: TypedGraphNode | null;
  x: number;
  y: number;
  visible: boolean;
}

const props = defineProps<Props>();

const getNodeIcon = (nodeType: string) => {
  switch (nodeType) {
    case 'DirectoryNode':
      return Folder;
    case 'FileNode':
      return File;
    case 'DefinitionNode':
      return Code;
    default:
      return Link;
  }
};

const getNodeColor = (nodeType: string) => {
  switch (nodeType) {
    case 'DirectoryNode':
      return 'text-amber-500';
    case 'FileNode':
      return 'text-emerald-500';
    case 'DefinitionNode':
      return 'text-violet-500';
    default:
      return 'text-muted-foreground';
  }
};

const relationshipTypeDisplay = computed(() => {
  if (!props.relationship) return '';

  switch (props.relationship.relationship_type) {
    case 'DIRECTORY_RELATIONSHIPS':
      return 'Directory Contains';
    case 'FILE_RELATIONSHIPS':
      return 'File Contains';
    case 'DEFINITION_RELATIONSHIPS':
      return 'Definition Reference';
    default:
      return props.relationship.relationship_type.replace(/_/g, ' ').toLowerCase();
  }
});

const relationshipColor = computed(() => {
  if (!props.relationship) return 'bg-muted';

  switch (props.relationship.relationship_type) {
    case 'DIRECTORY_RELATIONSHIPS':
      return 'bg-amber-500';
    case 'FILE_RELATIONSHIPS':
      return 'bg-emerald-500';
    case 'DEFINITION_RELATIONSHIPS':
      return 'bg-violet-500';
    default:
      return 'bg-muted';
  }
});

const shouldFlipTooltip = computed(() => {
  return props.y > 400; // Simple threshold instead of window height
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible && relationship && sourceNode && targetNode"
      class="fixed z-50 pointer-events-none"
      :style="{
        left: `${x + 10}px`,
        top: `${y - 10}px`,
        transform: shouldFlipTooltip ? 'translateY(-100%)' : 'translateY(0)',
      }"
    >
      <Card class="w-96 shadow-lg border-2 bg-background/95 backdrop-blur-sm">
        <CardHeader class="pb-3">
          <CardTitle class="flex items-center gap-2 text-base">
            <div
              class="flex items-center justify-center w-6 h-6 rounded-full"
              :class="relationshipColor"
            >
              <ArrowRight class="w-3 h-3 text-white" />
            </div>
            <span>{{ relationshipTypeDisplay }}</span>
            <Badge variant="outline" class="ml-auto text-xs"> Relationship </Badge>
          </CardTitle>
        </CardHeader>

        <CardContent class="pt-0 space-y-4">
          <!-- Source Node -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-sm font-medium">
              <component
                :is="getNodeIcon(sourceNode.node_type)"
                class="w-4 h-4"
                :class="getNodeColor(sourceNode.node_type)"
              />
              <span>Source</span>
            </div>
            <div class="ml-6 space-y-1">
              <div class="flex items-center gap-2">
                <span class="font-mono text-sm">{{ sourceNode.label }}</span>
                <Badge variant="secondary" class="text-xs">
                  {{ sourceNode.node_type.replace('Node', '') }}
                </Badge>
              </div>
              <div class="text-xs text-muted-foreground">
                {{
                  sourceNode.properties.path ||
                  (sourceNode.node_type === 'DefinitionNode' ? sourceNode.properties.fqn : '')
                }}
              </div>
            </div>
          </div>

          <Separator />

          <!-- Target Node -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-sm font-medium">
              <component
                :is="getNodeIcon(targetNode.node_type)"
                class="w-4 h-4"
                :class="getNodeColor(targetNode.node_type)"
              />
              <span>Target</span>
            </div>
            <div class="ml-6 space-y-1">
              <div class="flex items-center gap-2">
                <span class="font-mono text-sm">{{ targetNode.label }}</span>
                <Badge variant="secondary" class="text-xs">
                  {{ targetNode.node_type.replace('Node', '') }}
                </Badge>
              </div>
              <div class="text-xs text-muted-foreground">
                {{
                  targetNode.properties.path ||
                  (targetNode.node_type === 'DefinitionNode' ? targetNode.properties.fqn : '')
                }}
              </div>
            </div>
          </div>

          <!-- Relationship Properties -->
          <div
            v-if="relationship.properties && Object.keys(relationship.properties).length > 0"
            class="space-y-2"
          >
            <Separator />
            <div class="text-sm font-medium">Properties</div>
            <div class="ml-6 space-y-1">
              <div
                v-for="[key, value] in Object.entries(relationship.properties)"
                :key="key"
                class="flex items-center gap-2 text-xs"
              >
                <span class="text-muted-foreground">{{ key }}:</span>
                <span class="font-mono">{{ value }}</span>
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
