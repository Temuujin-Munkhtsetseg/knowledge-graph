import { createApp } from 'vue';
import { VueQueryPlugin, QueryClient } from '@tanstack/vue-query';
import App from './App.vue';
import { setupThemeListener } from '@/lib/theme-utils';
import './style.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      staleTime: 30000,
    },
  },
});

const app = createApp(App);
app.use(VueQueryPlugin, { queryClient });

// Set up theme change listener
setupThemeListener();

app.mount('#app');
