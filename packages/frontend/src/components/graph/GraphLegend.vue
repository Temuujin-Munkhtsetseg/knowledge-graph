<script setup lang="ts">
interface Props {
  getNodeColor: (nodeType: string) => string;
}

defineProps<Props>();
</script>

<template>
  <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs">
    <div class="flex items-center gap-1.5">
      <div
        class="w-3 h-3 rounded-full"
        :style="{ backgroundColor: getNodeColor('DirectoryNode') }"
      />
      <span>Directory</span>
    </div>
    <div class="flex items-center gap-1.5">
      <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: getNodeColor('FileNode') }" />
      <span>File</span>
    </div>
    <div class="flex items-center gap-1.5">
      <div
        class="w-3 h-3 rounded-full"
        :style="{ backgroundColor: getNodeColor('DefinitionNode') }"
      />
      <span>Definition</span>
    </div>
  </div>
</template>
