const OS = {
    windows: "Windows",
    linux: "Linux",
};

const SEPARATOR = navigator.userAgent.includes(OS.windows) ? "\\" : "/";
const SEPARATOR_EXP = new RegExp(/\\|\//);
const UNC = "\\\\";

export default class path {
    static join(...paths: string[]) {
        const components = paths
            .map((a) => this.split(a))
            .flat()
            .filter(Boolean);
        return this.joinPaths(components);
    }

    static joinPaths(paths: string[]) {
        if (!paths.length) return "";

        if (navigator.userAgent.includes(OS.windows)) {
            return paths.length > 1 ? paths.join(SEPARATOR) : paths[0] + SEPARATOR;
        } else {
            return SEPARATOR + paths.join(SEPARATOR);
        }
    }

    static extname(path: string | undefined) {
        if (!path) return "";

        const components = this.split(path);
        const lastComponent = components[components.length - 1];

        if (lastComponent.lastIndexOf(".") < 0) return "";

        return lastComponent.substring(lastComponent.lastIndexOf(".")).toLowerCase();
    }

    static basename(path: string | undefined) {
        if (!path) return "";

        const components = this.split(path);
        return components[components.length - 1];
    }

    static dirname(path: string | undefined) {
        if (!path) return "";

        const components = this.split(path);
        const rest = components.slice(0, components.length - 1);
        return this.joinPaths(rest);
    }

    static root(path: string | undefined) {
        if (!path) return "";
        const components = this.split(path);
        return components[0] + SEPARATOR;
    }

    static split(path: string | undefined) {
        if (!path) return [];
        const pattern = path.startsWith(UNC) ? new RegExp(/(?<!^|\\)\\/) : SEPARATOR_EXP;
        return path.split(pattern).filter(Boolean);
    }
}
