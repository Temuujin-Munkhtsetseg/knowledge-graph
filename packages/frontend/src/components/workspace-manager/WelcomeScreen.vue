<script setup lang="ts">
import { Folder, GitBranch, Activity } from 'lucide-vue-next';
import GitLabIcon from '@/components/icons/GitLabIcon.vue';
</script>

<template>
  <div class="max-w-4xl mx-auto space-y-6">
    <!-- Welcome Section with VS Code styling -->
    <div class="space-y-3">
      <div class="flex items-center gap-2">
        <GitLabIcon class="h-5 w-5 text-foreground" />
        <h2 class="text-lg font-medium text-foreground">GitLab Knowledge Graph</h2>
      </div>
      <p class="text-xs text-muted-foreground leading-relaxed max-w-2xl">
        Manage your workspaces and projects from the sidebar. Add new workspaces to get started with
        indexing and organizing your codebase.
      </p>
    </div>

    <!-- VS Code Style Feature Cards - More Compact -->
    <div class="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
      <!-- Quick Start Card -->
      <div
        class="group relative p-3 border border-border bg-card hover:bg-muted/30 transition-colors cursor-pointer rounded-sm"
      >
        <div class="flex items-start gap-2">
          <div class="flex h-6 w-6 items-center justify-center rounded bg-primary/10">
            <Folder class="h-3 w-3 text-primary" />
          </div>
          <div class="space-y-1 flex-1">
            <h3 class="text-xs font-medium text-foreground">Quick Start</h3>
            <p class="text-xs text-muted-foreground leading-relaxed">
              Add your first workspace using the + button in the sidebar to begin indexing
            </p>
          </div>
        </div>
      </div>

      <!-- Project Management Card -->
      <div
        class="group relative p-3 border border-border bg-card hover:bg-muted/30 transition-colors cursor-pointer rounded-sm"
      >
        <div class="flex items-start gap-2">
          <div class="flex h-6 w-6 items-center justify-center rounded bg-primary/10">
            <GitBranch class="h-3 w-3 text-primary" />
          </div>
          <div class="space-y-1 flex-1">
            <h3 class="text-xs font-medium text-foreground">Project Management</h3>
            <p class="text-xs text-muted-foreground leading-relaxed">
              View and manage all your projects in one centralized location
            </p>
          </div>
        </div>
      </div>

      <!-- Real-time Updates Card -->
      <div
        class="group relative p-3 border border-border bg-card hover:bg-muted/30 transition-colors cursor-pointer rounded-sm md:col-span-2 lg:col-span-1"
      >
        <div class="flex items-start gap-2">
          <div class="flex h-6 w-6 items-center justify-center rounded bg-primary/10">
            <Activity class="h-3 w-3 text-primary" />
          </div>
          <div class="space-y-1 flex-1">
            <h3 class="text-xs font-medium text-foreground">Real-time Updates</h3>
            <p class="text-xs text-muted-foreground leading-relaxed">
              Track indexing progress and status updates in real-time via event bus
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
