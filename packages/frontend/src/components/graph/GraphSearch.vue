<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useQuery } from '@tanstack/vue-query';
import { Search, X, Folder, File, Code, Loader2, Import } from 'lucide-vue-next';
import type { TypedGraphNode } from '@gitlab-org/gkg';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { apiClient } from '@/api/client';

interface Props {
  projectPath: string;
  workspaceFolderPath: string;
  visible: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'nodeSelected', node: TypedGraphNode): void;
}>();

const searchTerm = ref('');
const debouncedSearchTerm = ref('');

// Debounce search input
let debounceTimeout: number;
watch(searchTerm, (newTerm) => {
  clearTimeout(debounceTimeout);
  debounceTimeout = setTimeout(() => {
    debouncedSearchTerm.value = newTerm.trim();
  }, 200);
});

const {
  data: searchResults,
  isLoading: isSearchLoading,
  error: searchError,
} = useQuery({
  queryKey: ['graph-search', props.projectPath, props.workspaceFolderPath, debouncedSearchTerm],
  queryFn: () => {
    return apiClient.searchNodes(
      props.workspaceFolderPath,
      props.projectPath,
      debouncedSearchTerm.value,
      50,
    );
  },
  enabled: computed(
    () =>
      Boolean(props.projectPath) &&
      Boolean(props.workspaceFolderPath) &&
      Boolean(debouncedSearchTerm.value) &&
      props.visible,
  ),
});

const hasSearchTerm = computed(() => debouncedSearchTerm.value.length > 0);
const hasResults = computed(
  () => searchResults.value?.nodes && searchResults.value.nodes.length > 0,
);
const resultCount = computed(() => searchResults.value?.nodes?.length || 0);

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
      return File;
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

const getBadgeVariant = (nodeType: string) => {
  switch (nodeType) {
    case 'DirectoryNode':
      return 'default';
    case 'FileNode':
      return 'secondary';
    case 'DefinitionNode':
      return 'outline';
    case 'ImportedSymbolNode':
      return 'outline';
    default:
      return 'outline';
  }
};

const clearSearch = () => {
  searchTerm.value = '';
  debouncedSearchTerm.value = '';
};

const handleClose = () => {
  clearSearch();
  emit('close');
};

const handleNodeClick = (node: TypedGraphNode) => {
  emit('nodeSelected', node);
};

const highlightSearchTerm = (text: string, term: string) => {
  if (!term) return text;
  const regex = new RegExp(`(${term})`, 'gi');
  return text.replace(
    regex,
    '<mark class="bg-yellow-200 dark:bg-yellow-800 px-1 rounded">$1</mark>',
  );
};
</script>

<template>
  <div
    v-if="visible"
    class="fixed right-4 top-4 bottom-4 w-80 z-40 transition-all duration-300 ease-out"
  >
    <Card class="h-full flex flex-col shadow-lg border-2 bg-background/95 backdrop-blur-sm">
      <CardHeader class="flex-shrink-0 pb-3">
        <div class="flex items-center justify-between">
          <CardTitle class="flex items-center gap-2 text-lg">
            <Search class="h-5 w-5" />
            Search Nodes
          </CardTitle>
          <Button variant="ghost" size="sm" class="h-8 w-8 p-0" @click="handleClose">
            <X class="h-4 w-4" />
          </Button>
        </div>
        <div class="relative">
          <Input
            v-model="searchTerm"
            placeholder="Search by name, path, or FQN..."
            class="pr-8"
            autofocus
          />
          <Button
            v-if="searchTerm"
            variant="ghost"
            size="sm"
            class="absolute right-1 top-1/2 -translate-y-1/2 h-6 w-6 p-0"
            @click="clearSearch"
          >
            <X class="h-3 w-3" />
          </Button>
        </div>
      </CardHeader>

      <CardContent class="flex-1 overflow-hidden flex flex-col p-4">
        <!-- Loading State -->
        <div v-if="isSearchLoading" class="flex items-center justify-center py-8">
          <div class="flex items-center gap-2 text-muted-foreground">
            <Loader2 class="h-4 w-4 animate-spin" />
            <span>Searching...</span>
          </div>
        </div>

        <!-- Error State -->
        <div v-else-if="searchError" class="text-center py-8">
          <p class="text-destructive text-sm">{{ searchError.message }}</p>
        </div>

        <!-- Empty State -->
        <div v-else-if="!hasSearchTerm" class="text-center py-8 text-muted-foreground">
          <Search class="h-12 w-12 mx-auto mb-3 opacity-50" />
          <p class="text-sm">Enter a search term to find nodes</p>
          <p class="text-xs mt-1">Search across directories, files, and definitions</p>
        </div>

        <!-- No Results -->
        <div
          v-else-if="hasSearchTerm && !hasResults && !isSearchLoading"
          class="text-center py-8 text-muted-foreground"
        >
          <Search class="h-12 w-12 mx-auto mb-3 opacity-50" />
          <p class="text-sm">No results found</p>
          <p class="text-xs mt-1">Try a different search term</p>
        </div>

        <!-- Results -->
        <div v-else-if="hasResults" class="flex flex-col h-full">
          <div class="flex-shrink-0 mb-3">
            <p class="text-sm text-muted-foreground">
              {{ resultCount }} result{{ resultCount !== 1 ? 's' : '' }} found
            </p>
          </div>

          <div class="flex-1 overflow-y-auto space-y-2">
            <div
              v-for="node in searchResults?.nodes"
              :key="node.id"
              class="p-3 border rounded-lg cursor-pointer hover:bg-accent/50 transition-colors group"
              @click="handleNodeClick(node)"
            >
              <div class="flex items-start gap-3">
                <div class="flex-shrink-0 mt-0.5">
                  <component
                    :is="getNodeIcon(node.node_type)"
                    class="h-4 w-4"
                    :class="getNodeColor(node.node_type)"
                  />
                </div>

                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1">
                    <p
                      class="font-medium text-sm truncate"
                      v-html="highlightSearchTerm(node.label, debouncedSearchTerm)"
                    />
                    <Badge :variant="getBadgeVariant(node.node_type)" class="text-xs flex-shrink-0">
                      {{ node.node_type.replace('Node', '') }}
                    </Badge>
                  </div>

                  <div class="space-y-1">
                    <!-- Directory Node -->
                    <div v-if="node.node_type === 'DirectoryNode'">
                      <p
                        class="text-xs text-muted-foreground truncate"
                        v-html="highlightSearchTerm(node.properties.path, debouncedSearchTerm)"
                      />
                      <p
                        v-if="node.properties.repository_name"
                        class="text-xs text-muted-foreground"
                      >
                        {{ node.properties.repository_name }}
                      </p>
                    </div>

                    <!-- File Node -->
                    <div v-else-if="node.node_type === 'FileNode'">
                      <p
                        class="text-xs text-muted-foreground truncate"
                        v-html="highlightSearchTerm(node.properties.path, debouncedSearchTerm)"
                      />
                      <div class="flex items-center gap-2 mt-1">
                        <Badge v-if="node.properties.language" variant="outline" class="text-xs">
                          {{ node.properties.language }}
                        </Badge>
                        <Badge v-if="node.properties.extension" variant="outline" class="text-xs">
                          {{ node.properties.extension }}
                        </Badge>
                      </div>
                    </div>

                    <!-- Definition Node -->
                    <div v-else-if="node.node_type === 'DefinitionNode'">
                      <p
                        class="text-xs text-muted-foreground truncate font-mono"
                        v-html="highlightSearchTerm(node.properties.fqn, debouncedSearchTerm)"
                      />
                      <p
                        class="text-xs text-muted-foreground truncate"
                        v-html="highlightSearchTerm(node.properties.path, debouncedSearchTerm)"
                      />
                      <div class="flex items-center gap-2 mt-1">
                        <Badge variant="outline" class="text-xs">
                          {{ node.properties.definition_type }}
                        </Badge>
                        <span class="text-xs text-muted-foreground">
                          Line {{ node.properties.start_line }}
                        </span>
                      </div>
                    </div>

                    <!-- Imported Symbol Node -->
                    <div v-else-if="node.node_type === 'ImportedSymbolNode'">
                      <p
                        class="text-xs text-muted-foreground truncate font-mono"
                        v-html="highlightSearchTerm(node.label, debouncedSearchTerm)"
                      />
                      <p
                        class="text-xs text-muted-foreground truncate"
                        v-html="highlightSearchTerm(node.properties.path, debouncedSearchTerm)"
                      />
                      <div class="flex items-center gap-2 mt-1">
                        <Badge variant="outline" class="text-xs">
                          {{ node.properties.import_type }}
                        </Badge>
                        <span class="text-xs text-muted-foreground">
                          Line {{ node.properties.start_line }}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>

<style scoped>
/* Custom scrollbar for results */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: hsl(var(--border));
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--border) / 0.8);
}

/* Firefox scrollbar */
.overflow-y-auto {
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--border)) transparent;
}
</style>
