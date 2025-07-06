<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useColorMode } from '@vueuse/core';
import { Moon, Sun, Monitor, Activity } from 'lucide-vue-next';
import type { GkgEvent } from '@gitlab-org/gkg';
import WorkspaceManagerSidebar from './WorkspaceManagerSidebar.vue';
import WelcomeScreen from './WelcomeScreen.vue';
import { MainContent } from '@/components/content';
import GitLabIcon from '@/components/icons/GitLabIcon.vue';
import { SidebarProvider, SidebarInset, SidebarTrigger } from '@/components/ui/sidebar';
import { Separator } from '@/components/ui/separator';
import { Button } from '@/components/ui/button';
import DropdownMenu from '@/components/ui/dropdown-menu/DropdownMenu.vue';
import DropdownMenuContent from '@/components/ui/dropdown-menu/DropdownMenuContent.vue';
import DropdownMenuItem from '@/components/ui/dropdown-menu/DropdownMenuItem.vue';
import DropdownMenuTrigger from '@/components/ui/dropdown-menu/DropdownMenuTrigger.vue';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { useWorkspaceStream, useWorkspaces } from '@/hooks/api';

const isSmallScreen = ref(false);
const sidebarOpen = ref(true);

// Use @vueuse/core's useColorMode for better theme management
const mode = useColorMode();

const { startStream, stopStream, isConnected, lastEvent } = useWorkspaceStream();
const { data: workspacesData, isLoading: workspacesLoading } = useWorkspaces();

const hasWorkspaces = computed(() => {
  return workspacesData.value && workspacesData.value.workspaces.length > 0;
});

const checkScreenSize = () => {
  isSmallScreen.value = window.innerWidth < 768;
  if (isSmallScreen.value) {
    sidebarOpen.value = false;
  } else {
    sidebarOpen.value = true;
  }
};

onMounted(() => {
  checkScreenSize();
  window.addEventListener('resize', checkScreenSize);
});

onUnmounted(() => {
  window.removeEventListener('resize', checkScreenSize);
  stopStream();
});

// Start the SSE stream after initial workspace data is loaded
watch(
  () => workspacesLoading.value,
  (isLoading) => {
    if (!isLoading) {
      // Start the stream regardless of whether workspace data loaded successfully
      // This ensures the connection is established and we can get real-time updates
      startStream();
    }
  },
  { immediate: true },
);

// Helper function to get status from event
const getEventStatus = (event: GkgEvent | null): string => {
  if (!event) return 'idle';

  if (event.type === 'WorkspaceIndexing') {
    return event.payload.status.toLowerCase();
  }
  if (event.type === 'ProjectIndexing') {
    return event.payload.status.toLowerCase();
  }

  return 'idle';
};

const eventStatus = computed(() => getEventStatus(lastEvent.value));

const openGitLabRepository = () => {
  window.open('https://gitlab.com/gitlab-org/rust/knowledge-graph', '_blank');
};

const selectedProjectPath = ref<string | null>(null);

const handleOpenProject = (projectPath: string) => {
  selectedProjectPath.value = projectPath;
};
</script>

<template>
  <TooltipProvider>
    <div class="h-screen flex bg-background">
      <SidebarProvider :open="sidebarOpen" @update:open="sidebarOpen = $event">
        <WorkspaceManagerSidebar @open-project="handleOpenProject" />

        <SidebarInset class="flex-1 flex flex-col min-w-0">
          <!-- VS Code Style Header - More Compact -->
          <header
            class="flex h-9 shrink-0 items-center gap-2 border-b border-border bg-background px-3"
          >
            <SidebarTrigger class="-ml-1 h-5 w-5" />
            <Separator orientation="vertical" class="h-3" />
            <h1 class="text-xs font-medium text-foreground truncate flex-1">
              <span class="hidden sm:inline">Knowledge Graph Workspace Manager</span>
              <span class="sm:hidden">Workspace Manager</span>
            </h1>

            <!-- Connection Status Indicator -->
            <div class="flex items-center gap-2">
              <div class="flex items-center gap-1.5">
                <div
                  class="h-2 w-2 rounded-full transition-colors"
                  :class="{
                    'bg-green-500': isConnected,
                    'bg-red-500': !isConnected,
                  }"
                />
                <span class="text-xs text-muted-foreground hidden sm:inline">
                  {{ isConnected ? 'Connected' : 'Disconnected' }}
                </span>
              </div>

              <!-- Event Status -->
              <div v-if="lastEvent" class="flex items-center gap-1.5">
                <Activity class="h-3 w-3 text-muted-foreground" />
                <span class="text-xs text-muted-foreground capitalize">
                  {{ eventStatus }}
                </span>
              </div>
            </div>

            <!-- Theme Toggle with VS Code styling -->
            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <Button variant="ghost" size="sm" class="h-6 w-6 p-0 hover:bg-muted/60">
                  <Sun
                    class="h-3 w-3 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0"
                  />
                  <Moon
                    class="absolute h-3 w-3 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100"
                  />
                  <span class="sr-only">Toggle theme</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" class="w-28">
                <DropdownMenuItem class="text-xs h-6" @click="mode = 'light'">
                  <Sun class="mr-2 h-3 w-3" />
                  Light
                </DropdownMenuItem>
                <DropdownMenuItem class="text-xs h-6" @click="mode = 'dark'">
                  <Moon class="mr-2 h-3 w-3" />
                  Dark
                </DropdownMenuItem>
                <DropdownMenuItem class="text-xs h-6" @click="mode = 'auto'">
                  <Monitor class="mr-2 h-3 w-3" />
                  System
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            <!-- GitLab Repository Link -->
            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  variant="ghost"
                  size="sm"
                  class="h-6 w-6 p-0 hover:bg-muted/60"
                  @click="openGitLabRepository"
                >
                  <GitLabIcon class="h-3 w-3" />
                  <span class="sr-only">Open GitLab Repository</span>
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>View source code</p>
              </TooltipContent>
            </Tooltip>
          </header>

          <!-- VS Code Style Main Content - More Compact -->
          <main class="flex-1 overflow-auto p-4 bg-background">
            <MainContent
              v-if="hasWorkspaces"
              :last-event="lastEvent"
              :selected-project-path="selectedProjectPath"
            />
            <WelcomeScreen v-else />
          </main>
        </SidebarInset>
      </SidebarProvider>
    </div>
  </TooltipProvider>
</template>
