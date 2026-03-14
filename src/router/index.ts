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
    {
      path: "/project/:id/files",
      name: "file-browser",
      component: () => import("../views/FileBrowser.vue"),
      props: true,
    },
    {
      path: "/project/:id/a2",
      name: "a2",
      component: () => import("../views/A2View.vue"),
      props: true,
    },
    {
      path: "/project/:id/api-map",
      name: "api-map",
      component: () => import("../views/ApiMap.vue"),
      props: true,
    },
    {
      path: "/project/:id/snapshots",
      name: "snapshots",
      component: () => import("../views/SnapshotDiff.vue"),
      props: true,
    },
    {
      path: "/project/:id/semantic-search",
      name: "semantic-search",
      component: () => import("../views/SemanticSearch.vue"),
      props: true,
    },
    {
      path: "/project/:id/diagrams",
      name: "diagrams",
      component: () => import("../views/Diagrams.vue"),
      props: true,
    },
    {
      path: "/project/:id/workspaces",
      name: "workspaces",
      component: () => import("../views/Workspaces.vue"),
      props: true,
    },
    {
      path: "/project/:id/plugins",
      name: "plugins",
      component: () => import("../views/Plugins.vue"),
      props: true,
    },
    {
      path: "/project/:id/settings",
      name: "settings",
      component: () => import("../views/Settings.vue"),
      props: true,
    },
  ],
});

export default router;
