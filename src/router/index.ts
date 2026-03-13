import { createRouter, createWebHistory } from "vue-router";
import ProjectList from "../views/ProjectList.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "projects",
      component: ProjectList,
    },
    {
      path: "/project/:id",
      name: "project-dashboard",
      component: () => import("../views/ProjectDashboard.vue"),
      props: true,
    },
    {
      path: "/project/:id/runs",
      name: "run-center",
      component: () => import("../views/RunCenter.vue"),
      props: true,
    },
    {
      path: "/project/:id/context",
      name: "context-manager",
      component: () => import("../views/ContextManager.vue"),
      props: true,
    },
    {
      path: "/project/:id/health",
      name: "health-dashboard",
      component: () => import("../views/HealthDashboard.vue"),
      props: true,
    },
    {
      path: "/project/:id/search",
      name: "search",
      component: () => import("../views/SearchView.vue"),
      props: true,
    },
  ],
});

export default router;
