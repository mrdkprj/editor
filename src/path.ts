import { OS, SEPARATOR } from "./constants";

export class path {
    static join(...paths: string[]) {
        const components = navigator.userAgent.includes(OS.windows)
            ? paths
                  .map((a) => a.split(SEPARATOR))
                  .flat()
                  .filter(Boolean)
            : paths.map((a) => a.split(SEPARATOR)).flat();
        return components.join(SEPARATOR);
    }

    static extname(name: string | undefined) {
        if (!name) return "";

        if (name.lastIndexOf(".") < 0) return "";

        return name.substring(name.lastIndexOf(".")).toLowerCase();
    }

    static basename(path: string | undefined) {
        if (!path) return "";

        const components = path.split(SEPARATOR);
        return components[components.length - 1];
    }

    static dirname(path: string | undefined) {
        if (!path) return "";

        const components = path.split(SEPARATOR);
        const rest = components.slice(0, components.length - 1);
        return rest.join(SEPARATOR);
    }

    static root(path: string | undefined) {
        if (!path) return "";
        const components = path.split(SEPARATOR);
        return components[0] + SEPARATOR;
    }
}
