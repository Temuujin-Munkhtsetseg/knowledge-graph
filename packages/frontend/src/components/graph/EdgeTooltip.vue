<script setup lang="ts">
import { computed } from 'vue';
import { ArrowRight, Folder, File, Code, Link, Import } from 'lucide-vue-next';
import type { TypedGraphNode, GraphRelationship } from '@gitlab-org/gkg';
import { Card } from '@/components/ui/card';
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
    case 'ImportedSymbolNode':
      return Import;
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
    case 'ImportedSymbolNode':
      return 'text-blue-500';
    default:
      return 'text-muted-foreground';
  }
};

const relationshipTypeDisplay = computed(() => {
  if (!props.relationship) return '';

  switch (props.relationship.relationship_name) {
    case 'DIRECTORY_RELATIONSHIPS':
      return 'Directory Contains';
    case 'FILE_RELATIONSHIPS':
      return 'File Contains';
    case 'DEFINITION_RELATIONSHIPS':
      return 'Definition Reference';
    default:
      return props.relationship.relationship_name.replace(/_/g, ' ').toLowerCase();
  }
});

const relationshipColor = computed(() => {
  if (!props.relationship) return 'bg-muted';

  switch (props.relationship.relationship_name) {
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

const formatRelationshipType = (relationshipType: string) => {
  return relationshipType
    .replace(/_/g, ' ')
    .toLowerCase()
    .replace(/\b\w/g, (char) => char.toUpperCase())
    .replace(/"/g, '');
};
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible && relationship && sourceNode && targetNode"
      class="fixed z-50 pointer-events-none transition-all duration-200 ease-out"
      :style="{
        top: tooltipPosition.top,
        right: tooltipPosition.right,
      }"
    >
      <Card class="w-72 sm:w-80 shadow-lg border-2 bg-background/95 backdrop-blur-sm">
        <div class="flex flex-col p-3 space-y-2">
          <div class="flex items-center gap-2">
            <div
              class="flex items-center justify-center w-4 h-4 sm:w-5 sm:h-5 rounded-full flex-shrink-0"
              :class="relationshipColor"
            >
              <ArrowRight class="w-2 h-2 sm:w-2.5 sm:h-2.5 text-white" />
            </div>
            <span class="truncate min-w-0 flex-1 text-xs sm:text-sm font-medium">{{
              relationshipTypeDisplay
            }}</span>
            <Badge variant="outline" class="text-xs flex-shrink-0 px-1.5 py-0.5"
              >Relationship</Badge
            >
          </div>

          <!-- Source Node -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-xs sm:text-sm font-medium">
              <component
                :is="getNodeIcon(sourceNode.node_type)"
                class="w-3 h-3 sm:w-4 sm:h-4 flex-shrink-0"
                :class="getNodeColor(sourceNode.node_type)"
              />
              <span>Source</span>
            </div>
            <div class="ml-5 sm:ml-6 space-y-1">
              <div class="flex items-center gap-2">
                <span class="font-mono text-xs sm:text-sm truncate min-w-0">{{
                  sourceNode.label
                }}</span>
                <Badge variant="secondary" class="text-xs flex-shrink-0">
                  {{ sourceNode.node_type.replace('Node', '') }}
                </Badge>
              </div>
              <div class="text-xs text-muted-foreground break-words">
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
            <div class="flex items-center gap-2 text-xs sm:text-sm font-medium">
              <component
                :is="getNodeIcon(targetNode.node_type)"
                class="w-3 h-3 sm:w-4 sm:h-4 flex-shrink-0"
                :class="getNodeColor(targetNode.node_type)"
              />
              <span>Target</span>
            </div>
            <div class="ml-5 sm:ml-6 space-y-1">
              <div class="flex items-center gap-2">
                <span class="font-mono text-xs sm:text-sm truncate min-w-0">{{
                  targetNode.label
                }}</span>
                <Badge variant="secondary" class="text-xs flex-shrink-0">
                  {{ targetNode.node_type.replace('Node', '') }}
                </Badge>
              </div>
              <div class="text-xs text-muted-foreground break-words">
                {{
                  targetNode.properties.path ||
                  (targetNode.node_type === 'DefinitionNode' ? targetNode.properties.fqn : '')
                }}
              </div>
            </div>
          </div>

          <!-- Relationship type -->
          <div class="space-y-2">
            <Separator />
            <div class="ml-5 sm:ml-6 space-y-1 text-xs">
              <span class="text-muted-foreground">Relationship Type: </span>
              <span class="font-mono min-w-0">{{
                formatRelationshipType(relationship.relationship_type)
              }}</span>
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
