<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { useQuery } from '@tanstack/vue-query';
import { Network } from 'lucide-vue-next';
import type { TypedGraphNode, GraphRelationship } from '@gitlab-org/gkg';
import StyledPath from '../common/StyledPath.vue';
import GraphControls from './GraphControls.vue';
import GraphLegend from './GraphLegend.vue';
import GraphStateOverlay from './GraphStateOverlay.vue';
import NodeTooltip from './NodeTooltip.vue';
import EdgeTooltip from './EdgeTooltip.vue';
import GraphSearch from './GraphSearch.vue';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useGraphTheme } from '@/composables/useGraphTheme';
import { type GraphData, useGraphRenderer } from '@/composables/useGraphRenderer';
import { apiClient } from '@/api/client';

interface Props {
  projectPath: string;
  workspaceFolderPath: string;
}

const props = defineProps<Props>();

const isFullscreen = ref(false);
const isSearchVisible = ref(false);

// Tooltip state
const nodeTooltip = ref({
  visible: false,
  node: null as TypedGraphNode | null,
  x: 0,
  y: 0,
});

const edgeTooltip = ref({
  visible: false,
  relationship: null as GraphRelationship | null,
  sourceNode: null as TypedGraphNode | null,
  targetNode: null as TypedGraphNode | null,
  x: 0,
  y: 0,
});

// State for tracking clicked nodes and expanded graph data
const clickedNodes = ref(new Set<string>());

const currentGraphData = ref<GraphData>({
  nodes: [],
  relationships: [],
});

const { getNodeColor } = useGraphTheme();
const {
  graphContainer,
  initializeGraph,
  addNodesToGraph,
  centerOnNode,
  zoomIn,
  zoomOut,
  resetView,
  clearGraph,
  getRelationship,
} = useGraphRenderer();

const {
  data: initialGraphData,
  isLoading: isQueryLoading,
  error: queryError,
  refetch,
} = useQuery({
  queryKey: ['graph-initial', props.projectPath, props.workspaceFolderPath],
  queryFn: () => apiClient.fetchGraphData(props.workspaceFolderPath, props.projectPath),
  enabled: computed(() => Boolean(props.projectPath) && Boolean(props.workspaceFolderPath)),
});

const hasData = computed(() => currentGraphData.value.nodes.length > 0);
const nodeCount = computed(() => currentGraphData.value.nodes.length);
const relationshipCount = computed(() => currentGraphData.value.relationships.length);

// Graph event handlers
const handleNodeHover = (node: TypedGraphNode, event: { x: number; y: number }) => {
  nodeTooltip.value = {
    visible: true,
    node,
    x: event.x,
    y: event.y,
  };
};

const handleNodeLeave = () => {
  nodeTooltip.value.visible = false;
};

const handleEdgeHover = (
  edge: string,
  sourceNode: TypedGraphNode,
  targetNode: TypedGraphNode,
  event: { x: number; y: number },
  // eslint-disable-next-line max-params
) => {
  const relationship = getRelationship(edge);
  edgeTooltip.value = {
    visible: true,
    relationship,
    sourceNode,
    targetNode,
    x: event.x,
    y: event.y,
  };
};

const handleEdgeLeave = () => {
  edgeTooltip.value.visible = false;
};

/**
 * Double click on a node to expand the graph to include the node's neighbors.
 * This fetches the neighbors from the API,
 * adds them to the graph,
 * and centers the camera on the node.
 */
const handleNodeDoubleClick = async (node: TypedGraphNode) => {
  if (clickedNodes.value.has(node.id)) {
    return;
  }

  try {
    const neighborsData = await apiClient.fetchNodeNeighbors(
      props.workspaceFolderPath,
      props.projectPath,
      node.node_id,
      node.node_type,
      50, // limit
    );

    clickedNodes.value.add(node.id);

    const existingNodeIds = new Set(currentGraphData.value.nodes.map((n) => n.id));
    const existingRelationshipIds = new Set(currentGraphData.value.relationships.map((r) => r.id));

    const newNodes = neighborsData.nodes.filter((n) => !existingNodeIds.has(n.id));
    const newRelationships = neighborsData.relationships.filter(
      (r) => !existingRelationshipIds.has(r.id),
    );

    // Add new nodes and relationships to current graph data
    currentGraphData.value.nodes.push(...newNodes);
    currentGraphData.value.relationships.push(...newRelationships);

    await addNodesToGraph(newNodes, newRelationships);

    if (newNodes.length > 0) {
      centerOnNode(node.id);
    }
  } catch (error) {
    console.error('Failed to fetch node neighbors:', error);
  }
};

const initializeGraphWithData = async () => {
  if (currentGraphData.value.nodes.length === 0 || !currentGraphData.value.project_info) return;

  await initializeGraph(currentGraphData.value, {
    onNodeHover: handleNodeHover,
    onNodeLeave: handleNodeLeave,
    onEdgeHover: handleEdgeHover,
    onEdgeLeave: handleEdgeLeave,
    onNodeDoubleClick: handleNodeDoubleClick,
  });
};

const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value;
  setTimeout(() => {
    const sigma = useGraphRenderer().sigmaInstance();
    if (sigma) sigma.refresh();
  }, 100);
};

const toggleSearch = () => {
  isSearchVisible.value = !isSearchVisible.value;
};

const handleSearchClose = () => {
  isSearchVisible.value = false;
};

const handleNodeSelected = async (node: TypedGraphNode) => {
  try {
    if (!currentGraphData.value.project_info) {
      console.error('No project info available');
      return;
    }

    clearGraph();

    // Set current graph data to just the selected node
    currentGraphData.value = {
      nodes: [node],
      relationships: [],
      project_info: currentGraphData.value.project_info,
    };

    clickedNodes.value.clear();

    await initializeGraph(currentGraphData.value, {
      onNodeHover: handleNodeHover,
      onNodeLeave: handleNodeLeave,
      onEdgeHover: handleEdgeHover,
      onEdgeLeave: handleEdgeLeave,
      onNodeDoubleClick: handleNodeDoubleClick,
    });

    centerOnNode(node.id);
    isSearchVisible.value = false;
  } catch (error) {
    console.error('Failed to load selected node:', error);
  }
};

watch(
  () => props.projectPath,
  () => {
    clearGraph();
    currentGraphData.value = { nodes: [], relationships: [] };
    clickedNodes.value.clear();
    refetch();
  },
);

// Initialize currentGraphData when initialGraphData is loaded
watch(initialGraphData, (newData) => {
  if (newData) {
    currentGraphData.value = {
      nodes: [...newData.nodes],
      relationships: [...newData.relationships],
      project_info: newData.project_info,
    };
    initializeGraphWithData();
  }
});

onMounted(() => {
  if (initialGraphData.value) {
    currentGraphData.value = {
      nodes: [...initialGraphData.value.nodes],
      relationships: [...initialGraphData.value.relationships],
      project_info: initialGraphData.value.project_info,
    };
    initializeGraphWithData();
  }
});

onUnmounted(() => {
  clearGraph();
  currentGraphData.value = { nodes: [], relationships: [] };
});
</script>

<template>
  <Card class="w-full" :class="{ 'fixed inset-4 z-50': isFullscreen }">
    <CardHeader>
      <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div class="flex-1">
          <CardTitle class="flex items-center gap-2">
            <Network class="h-5 w-5" />
            Project Graph
          </CardTitle>
          <CardDescription class="mt-1">
            <ul class="space-y-1">
              <li class="flex items-center gap-2">
                <span>Workspace folder</span>
                <StyledPath :path="props.workspaceFolderPath" />
              </li>
              <li class="flex items-center gap-2">
                <span>Project</span>
                <StyledPath :path="props.projectPath" />
              </li>
            </ul>
          </CardDescription>
        </div>
        <GraphControls
          :is-loading="isQueryLoading"
          :has-data="hasData || false"
          @zoom-in="zoomIn"
          @zoom-out="zoomOut"
          @reset-view="resetView"
          @toggle-fullscreen="toggleFullscreen"
          @toggle-search="toggleSearch"
          @refresh="refetch"
        />
      </div>
    </CardHeader>
    <CardContent class="relative">
      <div
        ref="graphContainer"
        class="w-full bg-card border border-border rounded-lg overflow-hidden transition-all graph-container"
        :class="[isFullscreen ? 'h-[calc(100vh-12rem)]' : 'h-96', { 'opacity-20': isQueryLoading }]"
      />
      <GraphStateOverlay
        :is-loading="isQueryLoading"
        :error="queryError"
        :has-data="hasData || false"
        @refresh="refetch"
      />
      <div v-if="hasData" class="mt-4 space-y-3">
        <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-muted-foreground">
          <span>{{ nodeCount }} nodes</span>
          <span>{{ relationshipCount }} relationships</span>
        </div>
        <GraphLegend :get-node-color="getNodeColor" />
      </div>
    </CardContent>
  </Card>

  <!-- Tooltips -->
  <NodeTooltip
    :node="nodeTooltip.node"
    :x="nodeTooltip.x"
    :y="nodeTooltip.y"
    :visible="nodeTooltip.visible"
  />

  <EdgeTooltip
    :relationship="edgeTooltip.relationship"
    :source-node="edgeTooltip.sourceNode"
    :target-node="edgeTooltip.targetNode"
    :x="edgeTooltip.x"
    :y="edgeTooltip.y"
    :visible="edgeTooltip.visible"
  />

  <!-- Search Panel -->
  <GraphSearch
    :project-path="props.projectPath"
    :workspace-folder-path="props.workspaceFolderPath"
    :visible="isSearchVisible"
    @close="handleSearchClose"
    @node-selected="handleNodeSelected"
  />
</template>

<style scoped>
.fixed.inset-0 {
  background: hsl(var(--background)) !important;
  border: 1px solid hsl(var(--border));
  box-shadow:
    0 20px 25px -5px rgba(0, 0, 0, 0.1),
    0 10px 10px -5px rgba(0, 0, 0, 0.04);
  /* Ensure solid background overlay */
  backdrop-filter: blur(8px);
  position: relative;
  z-index: 50;
}

.fixed.inset-0::before {
  content: '';
  position: absolute;
  inset: 0;
  background: hsl(var(--background));
  opacity: 0.95;
  z-index: -1;
  border-radius: inherit;
}

:global(.dark) .fixed.inset-0 {
  box-shadow:
    0 20px 25px -5px rgba(0, 0, 0, 0.4),
    0 10px 10px -5px rgba(0, 0, 0, 0.2);
}

.graph-container {
  position: relative;
  background: linear-gradient(135deg, hsl(var(--card)) 0%, hsl(var(--muted) / 0.3) 100%);
}

.graph-container canvas {
  border-radius: inherit;
}

/* Ensure sigma canvas respects the container's background */
.graph-container canvas:first-child {
  background: transparent !important;
}

/* Dark mode gradient enhancement */
:global(.dark) .graph-container {
  background: linear-gradient(135deg, hsl(var(--card)) 0%, hsl(var(--accent) / 0.2) 100%);
}
</style>
