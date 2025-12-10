import { en } from "./en";
import { ja } from "./ja";

export const t = (key: keyof Mp.Labels) => {
    if (window.lang == "ja") {
        return ja[key];
    } else {
        return en[key];
    }
};
