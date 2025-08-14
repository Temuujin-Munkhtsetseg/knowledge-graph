<script setup lang="ts">
import { PopoverTrigger, type PopoverTriggerProps, useForwardProps } from 'reka-ui'

const props = defineProps<PopoverTriggerProps>()

const forwardedProps = useForwardProps(props)
</script>

<template>
  <PopoverTrigger
    data-slot="popover-trigger"
    v-bind="forwardedProps"
  >
    <slot />
  </PopoverTrigger>
  
</template>


