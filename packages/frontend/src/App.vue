<script setup lang="ts">
import { WorkspaceManagerLayout } from '@/components/workspace-manager';
import ErrorBoundary from '@/components/ErrorBoundary.vue';
</script>

<template>
  <ErrorBoundary>
    <WorkspaceManagerLayout />
  </ErrorBoundary>
</template>
