<script setup lang="ts">
import type { GkgEvent } from '@gitlab-org/gkg';

const apiContract: GkgEvent = {
  type: 'WorkspaceIndexing',
  payload: {
    status: 'Completed',
    completed_at: new Date().toISOString(),
    projects_indexed: [],
    workspace_folder_info: {
      workspace_folder_path: 'test',
      data_directory_name: 'test',
      status: 'test',
      last_indexed_at: new Date().toISOString(),
      project_count: 0,
    },
  },
};
console.log(apiContract);
</script>

<template>
  <div>Hello World</div>
</template>
