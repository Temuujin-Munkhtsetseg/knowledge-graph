import { ref, nextTick } from 'vue';
import Sigma from 'sigma';
import Graph from 'graphology';
import { circular } from 'graphology-layout';
import type { GraphInitialSuccessResponse, TypedGraphNode } from '@gitlab-org/gkg';
import { useGraphTheme } from './useGraphTheme';

interface MouseCoords {
  x: number;
  y: number;
}

export interface GraphEventCallbacks {
  onNodeHover?: (node: TypedGraphNode, event: MouseCoords) => void;
  onNodeLeave?: (node: TypedGraphNode) => void;
  onEdgeHover?: (
    edge: string,
    sourceNode: TypedGraphNode,
    targetNode: TypedGraphNode,
    event: MouseCoords,
  ) => void;
  onEdgeLeave?: (edge: string) => void;
  onNodeDoubleClick?: (node: TypedGraphNode, event: MouseCoords) => void;
}

export const useGraphRenderer = () => {
  const graphContainer = ref<HTMLElement>();
  let sigmaInstance: Sigma | null = null;
  let graphData: GraphInitialSuccessResponse | null = null;
  let nodeMap: Map<string, TypedGraphNode> = new Map();

  const { currentTheme, getNodeColor, getNodeSize, isDark } = useGraphTheme();

  const clearGraph = () => {
    if (sigmaInstance) {
      sigmaInstance.kill();
      sigmaInstance = null;
    }
    if (graphContainer.value) {
      graphContainer.value.innerHTML = '';
    }
    nodeMap.clear();
  };

  const initializeGraph = async (
    data: GraphInitialSuccessResponse,
    callbacks?: GraphEventCallbacks,
  ): Promise<Sigma | undefined> => {
    if (!graphContainer.value || !data) return undefined;

    clearGraph();
    await nextTick();

    graphData = data;
    nodeMap = new Map(data.nodes.map((node) => [node.id, node]));

    const graph = new Graph({ multi: false, allowSelfLoops: false });

    // Add nodes
    data.nodes.forEach((node: TypedGraphNode) => {
      graph.addNode(node.id, {
        label: node.label,
        size: getNodeSize(node.node_type),
        color: getNodeColor(node.node_type),
        nodeType: node.node_type,
        properties: node.properties,
        highlighted: false,
        x: Math.random() * 100,
        y: Math.random() * 100,
      });
    });

    // Add edges
    data.relationships.forEach((rel) => {
      if (graph.hasNode(rel.source) && graph.hasNode(rel.target)) {
        try {
          graph.addEdge(rel.source, rel.target, {
            size: 1,
            color: currentTheme.value.edge,
            type: 'arrow',
            highlighted: false,
            relationshipData: rel,
          });
        } catch (e) {
          // Ignore duplicate edges
        }
      }
    });

    // Apply layout
    if (graph.order > 0) {
      circular.assign(graph);
      if (graph.order > 1) {
        const { default: forceAtlas2 } = await import('graphology-layout-forceatlas2');
        const settings = forceAtlas2.inferSettings(graph);
        forceAtlas2.assign(graph, { iterations: 100, settings });
      }
    }

    // Create Sigma instance
    sigmaInstance = new Sigma(graph, graphContainer.value, {
      renderLabels: true,
      renderEdgeLabels: false,
      defaultNodeColor: currentTheme.value.node.default,
      defaultEdgeColor: currentTheme.value.edge,
      labelColor: { color: currentTheme.value.text },
      labelFont: 'Inter, system-ui, sans-serif',
      labelSize: 12,
      labelWeight: '500',
      enableEdgeEvents: true,
      nodeReducer: (_, nodeData) => {
        const res = { ...nodeData };
        if (nodeData.highlighted) {
          res.size = nodeData.size * 1.15;
          res.zIndex = 1;
        }
        return res;
      },
      edgeReducer: (_, edgeData) => {
        const res = { ...edgeData };
        if (edgeData.highlighted) {
          res.color = currentTheme.value.edgeHover;
          res.size = 2;
          res.zIndex = 1;
        }
        return res;
      },
      defaultDrawNodeHover: (context, nodeData, _settings) => {
        const size = (nodeData.size || 5) * 1.2;

        const ctx = context as CanvasRenderingContext2D;
        ctx.fillStyle = currentTheme.value.hoverBackground;
        ctx.beginPath();
        ctx.arc(nodeData.x, nodeData.y, size + 4, 0, Math.PI * 2);
        ctx.fill();

        ctx.strokeStyle = currentTheme.value.border;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(nodeData.x, nodeData.y, size + 1, 0, Math.PI * 2);
        ctx.stroke();

        ctx.fillStyle = nodeData.color;
        ctx.shadowColor = nodeData.color;
        ctx.shadowBlur = 8;
        ctx.beginPath();
        ctx.arc(nodeData.x, nodeData.y, size + 1, 0, Math.PI * 2);
        ctx.fill();

        ctx.shadowColor = 'transparent';
        ctx.shadowBlur = 0;
      },
      defaultDrawNodeLabel: (context, labelData, settings) => {
        if (!labelData.label) return;

        const size = settings.labelSize;
        const font = settings.labelFont;
        const weight = settings.labelWeight;

        const ctx = context as CanvasRenderingContext2D;
        ctx.font = `${weight} ${size}px ${font}`;
        ctx.fillStyle = currentTheme.value.text;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';

        ctx.shadowColor = isDark.value ? 'rgba(0, 0, 0, 0.9)' : 'rgba(255, 255, 255, 0.9)';
        ctx.shadowBlur = 4;
        ctx.shadowOffsetX = 1;
        ctx.shadowOffsetY = 1;

        ctx.fillText(labelData.label, labelData.x, labelData.y + labelData.size + 15);

        ctx.shadowColor = 'transparent';
        ctx.shadowBlur = 0;
        ctx.shadowOffsetX = 0;
        ctx.shadowOffsetY = 0;
      },
    });

    // Set up event handlers
    if (callbacks) {
      setupEventHandlers(callbacks);
    }

    return sigmaInstance;
  };

  function setupEventHandlers(callbacks: GraphEventCallbacks) {
    if (!sigmaInstance) return;

    let hoveredEdge: string | null = null;

    // Node events
    if (callbacks.onNodeHover) {
      sigmaInstance.on('enterNode', ({ node, event }) => {
        const nodeData = nodeMap.get(node);
        if (nodeData) {
          // Change cursor to pointer for clickable nodes
          if (graphContainer.value) {
            graphContainer.value.style.cursor = 'pointer';
          }
          callbacks.onNodeHover?.(nodeData, event);
        }
      });
    }

    if (callbacks.onNodeLeave) {
      sigmaInstance.on('leaveNode', ({ node }) => {
        const nodeData = nodeMap.get(node);
        if (nodeData) {
          // Reset cursor when leaving nodes
          if (graphContainer.value) {
            graphContainer.value.style.cursor = 'default';
          }
          callbacks.onNodeLeave?.(nodeData);
        }
      });
    }

    if (callbacks.onNodeDoubleClick) {
      sigmaInstance.on('doubleClickNode', ({ node, event }) => {
        const nodeData = nodeMap.get(node);
        if (nodeData) {
          callbacks.onNodeDoubleClick?.(nodeData, event);
        }
      });
    }

    // Edge events
    if (callbacks.onEdgeHover) {
      sigmaInstance.on('enterEdge', ({ edge, event }) => {
        hoveredEdge = edge;

        const sourceNode = sigmaInstance?.getGraph().source(edge);
        const targetNode = sigmaInstance?.getGraph().target(edge);

        if (sourceNode && targetNode) {
          const sourceData = nodeMap.get(sourceNode);
          const targetData = nodeMap.get(targetNode);

          if (sourceData && targetData) {
            callbacks.onEdgeHover?.(edge, sourceData, targetData, event);
          }
        }

        // Highlight the edge and connected nodes
        sigmaInstance?.getGraph().updateEdgeAttribute(edge, 'highlighted', () => true);
        if (sourceNode) {
          sigmaInstance?.getGraph().updateNodeAttribute(sourceNode, 'highlighted', () => true);
        }
        if (targetNode) {
          sigmaInstance?.getGraph().updateNodeAttribute(targetNode, 'highlighted', () => true);
        }
        sigmaInstance?.refresh();
      });
    }

    if (callbacks.onEdgeLeave) {
      sigmaInstance.on('leaveEdge', ({ edge }) => {
        if (hoveredEdge === edge) {
          hoveredEdge = null;
          callbacks.onEdgeLeave?.(edge);

          // Reset edge styling
          sigmaInstance?.getGraph().updateEdgeAttribute(edge, 'highlighted', () => false);

          // Reset connected nodes
          const sourceNode = sigmaInstance?.getGraph().source(edge);
          const targetNode = sigmaInstance?.getGraph().target(edge);

          if (sourceNode) {
            sigmaInstance?.getGraph().updateNodeAttribute(sourceNode, 'highlighted', () => false);
          }
          if (targetNode) {
            sigmaInstance?.getGraph().updateNodeAttribute(targetNode, 'highlighted', () => false);
          }

          sigmaInstance?.refresh();
        }
      });
    }
  }

  const zoomIn = () => sigmaInstance?.getCamera().animatedZoom({ duration: 200 });
  const zoomOut = () => sigmaInstance?.getCamera().animatedUnzoom({ duration: 200 });
  const resetView = () => sigmaInstance?.getCamera().animatedReset({ duration: 200 });
  const centerOnNode = (nodeId: string) => {
    if (!sigmaInstance) return;

    const graph = sigmaInstance.getGraph();
    if (!graph.hasNode(nodeId)) return;

    const camera = sigmaInstance.getCamera();
    const currentState = camera.getState();

    // Temporarily move the node to center for reset calculation
    const nodeAttrs = graph.getNodeAttributes(nodeId);
    const originalX = nodeAttrs.x;
    const originalY = nodeAttrs.y;

    // Move node to origin temporarily
    graph.setNodeAttribute(nodeId, 'x', 0);
    graph.setNodeAttribute(nodeId, 'y', 0);

    // Use animatedReset to center on origin (where our node now is)
    camera
      .animatedReset({ duration: 300 })
      .then(() => {
        // Restore original node position
        graph.setNodeAttribute(nodeId, 'x', originalX);
        graph.setNodeAttribute(nodeId, 'y', originalY);

        // Get camera position after reset
        const afterResetState = camera.getState();

        // Now adjust camera ratio back to what user had
        return camera.animate(
          {
            x: afterResetState.x,
            y: afterResetState.y,
            ratio: currentState.ratio,
            angle: afterResetState.angle,
          },
          { duration: 100 },
        );
      })
      .then(() => {
        return undefined;
      })
      .catch(() => {
        // Restore node position even if animation fails
        graph.setNodeAttribute(nodeId, 'x', originalX);
        graph.setNodeAttribute(nodeId, 'y', originalY);
        return undefined;
      });
  };
  const refresh = () => sigmaInstance?.refresh();

  const addNodesToGraph = async (
    nodes: TypedGraphNode[],
    relationships: GraphInitialSuccessResponse['relationships'],
  ) => {
    if (!sigmaInstance || !graphData) return;

    const graph = sigmaInstance.getGraph();

    // Add new nodes to the existing graph
    nodes.forEach((node: TypedGraphNode) => {
      if (!graph.hasNode(node.id)) {
        // Position new nodes near existing nodes to avoid them being too far away
        const existingNodes = graph.nodes();
        let x = Math.random() * 100;
        let y = Math.random() * 100;

        if (existingNodes.length > 0) {
          // Position near a random existing node
          const randomExisting = existingNodes[Math.floor(Math.random() * existingNodes.length)];
          const existingAttrs = graph.getNodeAttributes(randomExisting);
          x = existingAttrs.x + (Math.random() - 0.5) * 50;
          y = existingAttrs.y + (Math.random() - 0.5) * 50;
        }

        graph.addNode(node.id, {
          label: node.label,
          size: getNodeSize(node.node_type),
          color: getNodeColor(node.node_type),
          nodeType: node.node_type,
          properties: node.properties,
          highlighted: false,
          x,
          y,
        });

        // Add to our node map
        nodeMap.set(node.id, node);
      }
    });

    // Add new relationships
    relationships.forEach((rel) => {
      if (graph.hasNode(rel.source) && graph.hasNode(rel.target)) {
        try {
          graph.addEdge(rel.source, rel.target, {
            size: 1,
            color: currentTheme.value.edge,
            type: 'arrow',
            highlighted: false,
            relationshipData: rel,
          });
        } catch (e) {
          // Ignore duplicate edges
        }
      }
    });

    // Apply minimal layout adjustment for new nodes only
    if (nodes.length > 0) {
      try {
        const { default: forceAtlas2 } = await import('graphology-layout-forceatlas2');
        const settings = forceAtlas2.inferSettings(graph);
        // Run fewer iterations to preserve existing layout
        forceAtlas2.assign(graph, { iterations: 20, settings });
        sigmaInstance?.refresh();
      } catch {
        sigmaInstance?.refresh();
      }
    } else {
      sigmaInstance?.refresh();
    }
  };

  const getRelationship = (edgeId: string) => {
    if (!sigmaInstance || !graphData) return null;

    const edgeData = sigmaInstance.getGraph().getEdgeAttributes(edgeId);
    return edgeData.relationshipData || null;
  };

  return {
    graphContainer,
    sigmaInstance: () => sigmaInstance,
    clearGraph,
    initializeGraph,
    addNodesToGraph,
    zoomIn,
    zoomOut,
    resetView,
    centerOnNode,
    refresh,
    getRelationship,
  };
};
