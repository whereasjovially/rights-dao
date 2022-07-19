import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router';
import Home from '../views/home/Home.vue';
import errors from './modules/errors';
import persons from "./modules/person";
import post from "./modules/post";

const routes: Array<RouteRecordRaw> = [
    {
        path: '/',
        name: 'Home',
        component: Home,
    },
    ...errors,
    ...persons,
    ...post,
    {
        path: '/:catchAll(.*)',
        redirect: '/error/404',
    },
];

const router = createRouter({
    history: createWebHistory('/'),
    routes,
});

export default router;
