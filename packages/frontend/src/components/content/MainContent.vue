<script setup lang="ts">
import type { GkgEvent } from '@gitlab-org/gkg';
import { ExplorerView } from '@/components/explorer';
import { RecentActivity } from '@/components/workspace-manager';

interface Props {
  lastEvent: GkgEvent | null;
  selectedProjectPath: string | null;
}

const props = defineProps<Props>();
</script>

<template>
  <div class="max-w-6xl mx-auto space-y-8">
    <!-- Main Explorer View -->
    <ExplorerView :selected-project-path="props.selectedProjectPath" />

    <!-- Recent Activity Section -->
    <div class="border-t border-border pt-8">
      <RecentActivity :last-event="props.lastEvent" />
    </div>
  </div>
</template>
