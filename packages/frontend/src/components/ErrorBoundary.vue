<script setup lang="ts">
import { ref, provide, onErrorCaptured } from 'vue';
import { Button } from '@/components/ui/button';

const error = ref<Error | null>(null);
const hasError = ref(false);

const resetError = () => {
  error.value = null;
  hasError.value = false;
};

const refreshPage = () => {
  // eslint-disable-next-line no-restricted-globals
  location.reload();
};

onErrorCaptured((err) => {
  error.value = err;
  hasError.value = true;
  return false; // Prevent error from propagating
});

provide('resetError', resetError);
</script>

<template>
  <div
    v-if="hasError"
    class="min-h-screen bg-background flex items-center justify-center"
    role="alert"
    aria-live="assertive"
  >
    <div class="max-w-md w-full p-6 bg-card border border-border rounded-lg shadow-lg text-center">
      <div class="text-destructive mb-4" aria-hidden="true">
        <svg class="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.5 0L4.268 18.5c-.77.833.192 2.5 1.732 2.5z"
          />
        </svg>
      </div>

      <h2 id="error-title" class="text-xl font-semibold text-foreground mb-2">
        Something went wrong
      </h2>

      <p class="text-muted-foreground mb-4" aria-describedby="error-title">
        An unexpected error occurred. Please try refreshing the page or contact the knowledge graph
        team if the problem persists.
      </p>

      <div
        class="text-sm text-muted-foreground mb-4 p-3 bg-muted rounded font-mono text-left"
        role="region"
        aria-label="Error details"
      >
        {{ error?.message || 'Unknown error' }}
      </div>

      <div class="flex gap-2 justify-center" role="group" aria-label="Error recovery actions">
        <Button variant="outline" aria-label="Try to recover from the error" @click="resetError">
          Try Again
        </Button>

        <Button aria-label="Refresh the entire page" @click="refreshPage"> Refresh Page </Button>
      </div>
    </div>
  </div>

  <slot v-else />
</template>
