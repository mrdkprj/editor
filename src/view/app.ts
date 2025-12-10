import "./view.css";
import "../common.css";
import { mount } from "svelte";
import View from "./View.svelte";

const app = mount(View, {
    target: document.body,
});

export default app;
