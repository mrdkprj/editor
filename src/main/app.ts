import "../common.css";
import { mount } from "svelte";
import Main from "./Main.svelte";

const app = mount(Main, {
    target: document.body,
});

export default app;
