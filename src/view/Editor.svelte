<script lang="ts">
    import { appState, dispatch, textState, settings } from "./appStateReducer.svelte";
    import type monaco from "monaco-editor";
    import { onMount } from "svelte";
    import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
    import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
    import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
    import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
    import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
    import { path } from "../path";
    import { IPC } from "../ipc";
    import Deferred from "../deferred";
    import { BROWSER_SHORTCUT_KEYS, LANGUAGES, LINE_ENDINGS, SINGLE_BROWSER_SHORTCUT_KEYS } from "../constants";
    import { KeyCode, KeyMod } from "monaco-editor";
    import util from "../util";

    let {
        startLine,
        save,
        openNewWindow,
        getClipboardUrls,
        getClipboardText,
        unwatch,
        startGrep,
    }: {
        startLine?: Mp.Position;
        getClipboardUrls: () => Promise<Mp.PasteData>;
        getClipboardText: () => Promise<string>;
        save: (saveAs: boolean) => Promise<boolean>;
        openNewWindow: (filePath: string, grepRequest?: Mp.GrepRequest, position?: Mp.Position) => Promise<void>;
        unwatch: () => void;
        startGrep: () => void;
    } = $props();

    const ipc = new IPC("View");
    const DARK = "vs-dark";
    const LIGHT = "vs";
    const headerLineCount = 9;
    const decorationMap: { [key: string]: monaco.editor.IModelDeltaDecoration } = {};
    const matchRegexp = util.isWin() ? new RegExp(/(^(?=.*\\).*)\(([0-9]*),([0-9]*)\)/) : new RegExp(/(^(?=.*\/).*)\(([0-9]*),([0-9]*)\)/);

    let root: HTMLDivElement;
    let editor: monaco.editor.IStandaloneCodeEditor;
    let Monaco: typeof monaco;
    let model: monaco.editor.ITextModel;
    let watchDialogPromise: Deferred<Mp.WatchConfirmEvent> | null;
    let state: monaco.editor.ICodeEditorViewState | null = null;
    let supressChangeDetection = false;

    const onDialogEvent = (open: boolean) => {
        if (open) {
            state = editor.saveViewState();
        } else {
            editor.focus();
            editor.restoreViewState(state);
        }
    };

    const onEditorKeydown = async (e: KeyboardEvent) => {
        if ((e.ctrlKey && BROWSER_SHORTCUT_KEYS.includes(e.key)) || SINGLE_BROWSER_SHORTCUT_KEYS.includes(e.key)) {
            e.preventDefault();
            e.stopPropagation();
        }

        if (e.ctrlKey && e.key == "v") {
            const data = await getClipboardUrls();
            if (!data.fullPaths.length) return;
            const range = editor.getSelection()!;
            const edit = { identifier: "", range, text: data.fullPaths.join("\n"), forceMoveMarkers: true };
            editor.executeEdits("", [edit]);
            return;
        }

        if (e.ctrlKey) {
            switch (e.key) {
                case "j":
                    editor.trigger("", "editor.action.gotoLine", {});
                    return;
                case "s":
                    await requestSave(false);
                    return;
                case "r":
                    editor.getAction("editor.action.startFindReplaceAction")?.run();
                    return;
                case "/":
                    if ($appState.mode != "grep") {
                        await editor.getAction("editor.action.commentLine")?.run();
                    }
                    return;
                case "g":
                    startGrep();
                    return;
            }
        }
    };

    const paste = async () => {
        const text = await getClipboardText();
        if (text) {
            const range = editor.getSelection()!;
            const edit = { identifier: "", range, text: text, forceMoveMarkers: true };
            editor.executeEdits("", [edit]);
        }
    };

    const requestSave = async (saveAs: boolean) => {
        dispatch({ type: "suspendWatch", value: true });

        await editor.getAction("editor.action.formatDocument")?.run();
        const currentPath = $appState.fullPath;
        const saved = await save(saveAs);

        if (!saved) {
            dispatch({ type: "suspendWatch", value: false });
        }

        if (saved && currentPath != $appState.fullPath) {
            const state = editor.saveViewState();
            updateModel();
            editor.restoreViewState(state);
        }
    };

    const onWatchEvent = async (e: Mp.WatchEvent) => {
        if ($appState.suspendWatch) {
            dispatch({ type: "suspendWatch", value: false });
            return;
        }

        if ($appState.fullPath == e.file_path) {
            dispatch({ type: "showWatchDialog", value: true });

            watchDialogPromise = new Deferred();
            const result = await watchDialogPromise.promise;

            if (result.doNotNotify) {
                unwatch();
            }

            if (result.applyChange) {
                const state = editor.saveViewState();
                editor.setValue(e.content);
                textState.encoding = e.encoding;
                editor.restoreViewState(state);
            }
        }
    };

    const resolvePromise = async (result: Mp.WatchConfirmEvent) => {
        if (!watchDialogPromise) return;

        watchDialogPromise.resolve(result);
        watchDialogPromise = null;
    };

    const onDblClick = async (e: MouseEvent) => {
        if ($appState.mode != "grep") return;

        e.preventDefault();
        const position = editor.getPosition();
        if (!position) return;

        const text = model.getLineContent(position.lineNumber);
        const matched = matchRegexp.exec(text);
        if (!matched || matched.length != 4) return;

        const path = matched[1];
        const x = Number(matched[2]);
        const y = Number(matched[3]);
        const is_file = await util.is_file(path);
        if (is_file) {
            openNewWindow(path, undefined, { x, y });
        }
    };

    const handleContextMenuEvent = async (e: Mp.ContextMenuEvent) => {
        switch (e.id) {
            case "Save":
                await requestSave(false);
                break;
            case "SaveAs":
                await requestSave(true);
                break;

            case "Theme":
                const theme = e.value == "dark" ? DARK : LIGHT;
                editor.updateOptions({ theme });
                break;

            case "Undo":
                await model.undo();
                break;
            case "Redo":
                await model.redo();
                break;

            case "Copy": {
                editor.trigger("", "editor.action.clipboardCopyAction", {});
                break;
            }
            case "Cut": {
                editor.trigger("", "editor.action.clipboardCutAction", {});
                break;
            }
            case "Paste": {
                /* Can"t trigger clipboardPasteAction */
                // editor.trigger("", "editor.action.clipboardPasteAction", {});
                paste();
                break;
            }

            case "Search":
                editor.getAction("actions.find")?.run();
                break;
            case "Replace":
                editor.getAction("editor.action.startFindReplaceAction")?.run();
                break;
            case "Grep":
                startGrep();
                break;

            case "ToggleLineComment":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.commentLine")?.run();
                break;
            case "ToggleBlockComment":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.blockComment")?.run();
                break;

            case "transformToLowercase":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.transformToLowercase")?.run();
                break;
            case "transformToUppercase":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.transformToUppercase")?.run();
                break;
            case "transformToSnakecase":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.transformToSnakecase")?.run();
                break;
            case "transformToCamelcase":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.transformToCamelcase")?.run();
                break;
            case "transformToPascalcase":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.transformToPascalcase")?.run();
                break;
            case "transformToTitlecase":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.transformToTitlecase")?.run();
                break;
            case "transformToKebabcase":
                if ($appState.mode == "grep") return;
                await editor.getAction("editor.action.transformToKebabcase")?.run();
                break;

            case "ShowLineNumber": {
                const option = editor.getOption(Monaco.editor.EditorOption.lineNumbers);
                if (option.renderType == Monaco.editor.RenderLineNumbersType.On) {
                    editor.updateOptions({ lineNumbers: "off" });
                } else {
                    editor.updateOptions({ lineNumbers: "on" });
                }
                break;
            }
            case "AutoIndent": {
                if ($appState.mode == "grep") return;
                const option = editor.getOption(Monaco.editor.EditorOption.autoIndent);
                const autoIndent = option == Monaco.editor.EditorAutoIndentStrategy.Advanced ? false : true;
                settings.preference[textState.textType].autoIndent = autoIndent;
                updateModel();
                break;
            }
            case "Wordwrap": {
                const option = editor.getOption(Monaco.editor.EditorOption.wordWrap);
                if (option == "on") {
                    editor.updateOptions({ wordWrap: "off" });
                } else {
                    editor.updateOptions({ wordWrap: "on" });
                }
                break;
            }
            case "renderWhitespace": {
                editor.updateOptions({ renderWhitespace: e.value as Mp.WhiteSpaceRenderMode });
                break;
            }
            case "lineHighlight": {
                const option = editor.getOption(Monaco.editor.EditorOption.renderLineHighlight);
                if (option == "line") {
                    editor.updateOptions({ renderLineHighlight: "none" });
                } else {
                    editor.updateOptions({ renderLineHighlight: "line" });
                }
                break;
            }

            case "indentSize": {
                model.updateOptions({ indentSize: Number(e.value) });
                break;
            }
            case "indentBySpaces": {
                model.updateOptions({ insertSpaces: !model.getOptions().insertSpaces });
                break;
            }
            case "fontSize": {
                editor.updateOptions({ fontSize: Number(e.value) });
                break;
            }

            case "Format": {
                if ($appState.mode == "grep") return;
                editor.trigger("", "editor.action.formatDocument", {});
                break;
            }
        }
    };

    const getContent = () => {
        const initial = model.getLineCount() == 1 ? "" : "\n";
        const header = `${initial}\nCondition:\t${$appState.grepRequest?.condition}\nType:\t${$appState.grepRequest?.file_type}\nLocation:\t${$appState.grepRequest?.start_directory}\nInclude Sub Directory:\t${$appState.grepRequest?.recursive}\nCase Sensitive:\t${$appState.grepRequest?.case_sensitive}\nRegexp:\t${$appState.grepRequest?.regexp}\nMatches:\t${$appState.grepResults.length}\n\n`;
        const content = header + $appState.grepResults.map((result) => `${result.full_path}(${result.ranges[0][0]},${result.line_number}): ${result.line.trimEnd()}`).join("\n");
        return content;
    };

    const highLight = (startLineNumber: number) => {
        if (!$appState.grepRequest) return;

        const maxColumn = $appState.grepResults.reduce((a, b) => (a.line.length > b.line.length ? a : b)).line.length * 100;
        /* Limit or defaulted to 999*/
        const limit = $appState.grepResults.map((r) => r.ranges.length).reduce((accum, cur) => accum + cur) + $appState.grepResults.length * 100;
        const matches = model.findMatches(
            $appState.grepRequest.condition,
            {
                startColumn: 0,
                endColumn: maxColumn,
                startLineNumber,
                endLineNumber: model.getLineCount() + 1,
            },
            $appState.grepRequest.regexp,
            $appState.grepRequest.case_sensitive,
            null,
            false,
            limit,
        );

        matches.forEach((match: monaco.editor.FindMatch): void => {
            const decoration = {
                range: match.range,
                options: {
                    isWholeLine: false,
                    inlineClassName: "highlight",
                },
            };
            const ids = model.deltaDecorations([], [decoration]);

            decorationMap[ids[0]] = decoration;
        });
    };

    const onGrepResults = () => {
        const lineCount = model.getLineCount();
        const content = getContent();
        const range: monaco.IRange = {
            endColumn: 1,
            endLineNumber: lineCount + 1,
            startColumn: 0,
            startLineNumber: lineCount + 1,
        };
        const id = { major: 1, minor: 1 };
        const op = { identifier: id, range, text: content, forceMoveMarkers: true };
        model.applyEdits([op]);
        editor.revealLine(lineCount + headerLineCount);
        highLight(lineCount);
    };

    const onEncodingChanged = () => {
        supressChangeDetection = true;
        const state = editor.saveViewState();

        if ($appState.fullPath) {
            editor.setValue($appState.content);
        } else {
            editor.pushUndoStop();
            const wholeRange: monaco.IRange = {
                startColumn: 1,
                endColumn: model.getLineMaxColumn(model.getLineCount()),
                startLineNumber: 1,
                endLineNumber: model.getLineCount(),
            };
            const removeAll = { identifier: "removeAll", range: wholeRange, text: null, forceMoveMarkers: false };
            editor.executeEdits("", [removeAll]);
            editor.popUndoStop();

            const range: monaco.IRange = {
                startColumn: 1,
                endColumn: 1,
                startLineNumber: 1,
                endLineNumber: 1,
            };
            const insert = { identifier: "insert", range, text: $appState.content, forceMoveMarkers: true };
            editor.executeEdits("", [insert]);
        }
        editor.restoreViewState(state);
        supressChangeDetection = false;
    };

    const updateModel = () => {
        const state = editor.saveViewState();
        model.dispose();
        editor.dispose();
        createEditor();
        editor.restoreViewState(state);
    };

    const createEditor = async () => {
        // @ts-ignore
        self.MonacoEnvironment = {
            getWorker: function (_moduleId: any, label: string) {
                if (label === "json") {
                    return new jsonWorker();
                }
                if (label === "css" || label === "scss" || label === "less") {
                    return new cssWorker();
                }
                if (label === "html" || label === "handlebars" || label === "razor") {
                    return new htmlWorker();
                }
                if (label === "typescript" || label === "javascript") {
                    return new tsWorker();
                }
                return new editorWorker();
            },
        };

        const content = $appState.content;

        model = Monaco.editor.createModel(content, undefined, Monaco.Uri.file(path.basename($appState.fullPath)));
        const language = model.getLanguageId();
        const isPlainText = $appState.mode == "grep" || language == "plaintext";
        textState.textType = isPlainText ? "plain" : "code";

        model.updateOptions({ indentSize: settings.preference[textState.textType].indentSize, insertSpaces: settings.preference[textState.textType].indentBySpaces });

        editor = Monaco.editor.create(root, {
            model,
            theme: $appState.theme == "dark" ? DARK : LIGHT,
            hover: { enabled: false },
            contextmenu: false,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            automaticLayout: true,
            emptySelectionClipboard: false,
            suggest: isPlainText ? { showWords: false } : { showWords: true },
            autoIndent: settings.preference[textState.textType].autoIndent ? "advanced" : "none",
            folding: !isPlainText,
            wordWrap: settings.preference[textState.textType].wordWrap ? "on" : "off",
            /* Prevent wordwrap at space */
            wordWrapBreakAfterCharacters: "",
            /* Use "ctrlCmd" because "alt" blocks columnSelection */
            multiCursorModifier: "ctrlCmd",
            copyWithSyntaxHighlighting: true,
            unicodeHighlight: {
                nonBasicASCII: !isPlainText,
                ambiguousCharacters: !isPlainText,
                includeComments: !isPlainText,
                includeStrings: !isPlainText,
                invisibleCharacters: !isPlainText,
            },
            renderControlCharacters: !isPlainText,
            trimAutoWhitespace: false,
            renderValidationDecorations: "off",
            showUnused: false,
            showDeprecated: false,
            trimWhitespaceOnDelete: false,
            occurrencesHighlight: isPlainText ? "off" : "singleFile",
            renderWhitespace: settings.preference[textState.textType].renderWhitespace,
            guides: { indentation: false },
            formatOnType: false,
            renderLineHighlight: settings.preference[textState.textType].lineHighlight ? "line" : "none",
            find: { seedSearchStringFromSelection: "selection" },
        });

        if (startLine) {
            editor.revealLine(startLine.y, Monaco.editor.ScrollType.Immediate);
            editor.setPosition({ column: startLine.x, lineNumber: startLine.y });
        }

        dispatch({ type: "lineEnding", value: LINE_ENDINGS[model.getEndOfLineSequence()] });
        const languageName = language in LANGUAGES ? LANGUAGES[language] : language.charAt(0).toUpperCase() + language.slice(1);
        dispatch({ type: "language", value: languageName });

        editor.onDidChangeCursorPosition((e) => {
            dispatch({ type: "cusorPosition", value: { line: e.position.lineNumber, column: e.position.column } });
        });

        editor.focus();

        model.onDidChangeContent((e) => {
            if (supressChangeDetection) return;

            if ($appState.mode == "grep") {
                /* To prevent broken decorations, remove and restore decoration */
                e.changes.forEach((change) => {
                    model.getDecorationsInRange(change.range).forEach((lineDecoration) => {
                        if (lineDecoration.id in decorationMap) {
                            const decoration = decorationMap[lineDecoration.id];
                            model.deltaDecorations([lineDecoration.id], [decoration]);
                        }
                    });
                });
            }

            if ($appState.isDirty && !model.canUndo()) {
                dispatch({ type: "isDirty", value: false });
            }
            if (!$appState.isDirty && model.canUndo()) {
                dispatch({ type: "isDirty", value: true });
            }

            dispatch({ type: "content", value: editor.getValue({ preserveBOM: true, lineEnding: $appState.lineEnding }) });
        });
    };

    const init = async () => {
        Monaco = await import("monaco-editor");
        Monaco.editor.addKeybindingRules([
            {
                keybinding: KeyMod.CtrlCmd | KeyCode.KeyG,
                command: "",
            },
            {
                keybinding: KeyMod.CtrlCmd | KeyCode.KeyI,
                command: "",
            },

            {
                keybinding: KeyMod.CtrlCmd | KeyCode.KeyD,
                command: "",
            },
            {
                keybinding: KeyCode.F1,
                command: "",
            },
        ]);

        Monaco.languages.registerCompletionItemProvider("html", {
            triggerCharacters: [">"],
            provideCompletionItems: (model: monaco.editor.ITextModel, position: monaco.Position) => {
                const codePre: string = model.getValueInRange({
                    startLineNumber: position.lineNumber,
                    startColumn: 1,
                    endLineNumber: position.lineNumber,
                    endColumn: position.column,
                });

                const tag = codePre.match(/.*<(\w+)>$/)?.[1];

                if (!tag) {
                    return undefined;
                }

                const word = model.getWordUntilPosition(position);

                return {
                    suggestions: [
                        {
                            label: `</${tag}>`,
                            kind: Monaco.languages.CompletionItemKind.EnumMember,
                            insertTextRules: Monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            insertText: `$1</${tag}>`,
                            range: {
                                startLineNumber: position.lineNumber,
                                endLineNumber: position.lineNumber,
                                startColumn: word.startColumn,
                                endColumn: word.endColumn,
                            },
                        },
                    ],
                };
            },
        });
        await createEditor();
    };

    onMount(() => {
        init();
        ipc.receive("load", updateModel);
        ipc.receive("contextmenu_event", handleContextMenuEvent);
        ipc.receive("watch_event", onWatchEvent);
        ipc.receive("watch_confirm_event", resolvePromise);
        ipc.receive("dialog", onDialogEvent);
        ipc.receive("grep_end", onGrepResults);
        ipc.receive("encoding_changed", onEncodingChanged);

        return () => {
            ipc.release();
            model.dispose();
            editor.dispose();
        };
    });
</script>

<div
    id="root"
    class:disable-unexpected-closing-bracket={$appState.mode == "grep" || $appState.language == "Plain Text"}
    bind:this={root}
    onkeydown={onEditorKeydown}
    ondblclick={onDblClick}
    role="button"
    tabindex="-1"
></div>
