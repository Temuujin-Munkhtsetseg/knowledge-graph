<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { Network, Search } from 'lucide-vue-next';
import { GraphVisualization } from '@/components/graph';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { useWorkspaces } from '@/hooks/api';

interface Props {
  selectedProjectPath?: string | null;
}

const props = defineProps<Props>();

const searchQuery = ref('');
const selectedProject = ref<string | null>(null);
const isSearchOpen = ref(false);
const selectedIndex = ref(-1);

const { data: workspacesData } = useWorkspaces();

const availableProjects = computed(() => {
  if (!workspacesData.value?.workspaces) return [];

  return workspacesData.value.workspaces
    .flatMap(
      (workspace) =>
        workspace.projects?.map((project) => ({
          workspace: workspace.workspace_info,
          project,
          displayName: project.project_path || '',
          workspacePath: workspace.workspace_info?.workspace_folder_path || '',
          projectPath: project.project_path || '',
          isIndexed: project.status?.toLowerCase() === 'indexed',
        })) || [],
    )
    .filter((item) => item.isIndexed);
});

const filteredProjects = computed(() => {
  if (!searchQuery.value.trim()) return availableProjects.value;

  const query = searchQuery.value.toLowerCase();

  return availableProjects.value.filter(
    (project) =>
      project.displayName.toLowerCase().includes(query) ||
      project.workspacePath.toLowerCase().includes(query),
  );
});

const defaultProject = computed(() => {
  if (availableProjects.value.length === 0) return null;
  return availableProjects.value[0];
});

const currentProject = computed(() => {
  if (!selectedProject.value) return defaultProject.value;
  return (
    availableProjects.value.find((p) => p.projectPath === selectedProject.value) ||
    defaultProject.value
  );
});

const formatProjectName = (path: string) => {
  const parts = path.split('/');
  return parts[parts.length - 1] || path;
};

const selectProject = (projectPath: string) => {
  selectedProject.value = projectPath;
  searchQuery.value = '';
  isSearchOpen.value = false;
  selectedIndex.value = -1;
};

const handleKeyDown = (event: KeyboardEvent) => {
  if (!isSearchOpen.value || filteredProjects.value.length === 0) return;

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      selectedIndex.value = Math.min(selectedIndex.value + 1, filteredProjects.value.length - 1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
      break;
    case 'Enter':
      event.preventDefault();
      if (selectedIndex.value >= 0) {
        selectProject(filteredProjects.value[selectedIndex.value].projectPath);
      }
      break;
    default:
      break;
  }
};

// Reset selection when filtering changes
watch(filteredProjects, () => {
  if (searchQuery.value.length > 0 && filteredProjects.value.length > 0) {
    selectedIndex.value = 0;
  } else {
    selectedIndex.value = -1;
  }
});

// Ensure the search is open when the user starts typing
watch(searchQuery, () => {
  if (searchQuery.value.length > 0) {
    isSearchOpen.value = true;
  }
});

// Watch for external project selection
watch(
  () => props.selectedProjectPath,
  (newProjectPath) => {
    if (newProjectPath) {
      selectedProject.value = newProjectPath;
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="space-y-6">
    <!-- Explorer Header -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <Network class="h-5 w-5 text-foreground" />
        <h2 class="text-lg font-medium text-foreground">Project Explorer</h2>
      </div>

      <div class="flex items-center gap-2">
        <div class="relative">
          <Popover :open="isSearchOpen" @update:open="(v) => (isSearchOpen = v)">
            <PopoverTrigger as-child>
              <div>
                <Search
                  class="absolute left-2 top-1/2 transform -translate-y-1/2 h-3 w-3 text-muted-foreground z-10"
                />

                <Input
                  v-model="searchQuery"
                  placeholder="Search projects..."
                  class="pl-7 h-8 w-64 text-xs cursor-text"
                  role="combobox"
                  :aria-expanded="isSearchOpen"
                  aria-haspopup="listbox"
                  @keydown="handleKeyDown"
                />
              </div>
            </PopoverTrigger>
            <PopoverContent
              align="start"
              class="mt-1 w-64 p-0 max-h-60 overflow-y-auto"
              role="listbox"
            >
              <div
                v-if="filteredProjects.length === 0"
                class="px-2 py-1.5 text-sm text-muted-foreground"
              >
                No projects found
              </div>
              <div
                v-for="(project, index) in filteredProjects"
                :key="project.projectPath"
                :class="[
                  'px-2 py-1.5 cursor-pointer outline-none hover:bg-accent hover:text-accent-foreground',
                  { 'bg-accent text-accent-foreground': selectedIndex === index },
                ]"
                role="option"
                :aria-selected="selectedIndex === index"
                @mousedown.prevent
                @click="selectProject(project.projectPath)"
              >
                <div class="flex flex-col items-start w-full">
                  <div class="font-medium text-sm">
                    {{ formatProjectName(project.projectPath) }}
                  </div>
                  <div class="text-xs text-muted-foreground truncate w-full">
                    {{ project.projectPath }}
                  </div>
                </div>
              </div>
            </PopoverContent>
          </Popover>
        </div>
      </div>
    </div>

    <!-- Project Selection -->
    <div v-if="availableProjects.length > 0" class="space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-medium text-foreground">Available Projects</h3>
        <span class="text-xs text-muted-foreground">
          {{ availableProjects.length }} indexed project{{
            availableProjects.length !== 1 ? 's' : ''
          }}
        </span>
      </div>

      <div class="flex flex-wrap gap-2">
        <Button
          v-for="project in availableProjects"
          :key="project.projectPath"
          :variant="currentProject?.projectPath === project.projectPath ? 'default' : 'outline'"
          size="sm"
          class="h-7 text-xs"
          @click="selectProject(project.projectPath)"
        >
          {{ formatProjectName(project.projectPath) }}
        </Button>
      </div>
    </div>

    <!-- Graph Visualization -->
    <div v-if="currentProject" class="space-y-4">
      <GraphVisualization
        :key="currentProject.projectPath"
        :project-path="currentProject.projectPath"
        :workspace-folder-path="currentProject.workspacePath"
      />
    </div>

    <!-- Empty State -->
    <div v-else class="flex items-center justify-center h-64">
      <Card class="max-w-md">
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <Network class="h-5 w-5" />
            No Projects Available
          </CardTitle>
          <CardDescription>
            No indexed projects found. Make sure you have workspaces added and indexed.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p class="text-sm text-muted-foreground">
            Add workspaces from the sidebar and wait for indexing to complete to see project graphs.
          </p>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
