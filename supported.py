
from os import listdir, getcwd
from os.path import isfile, join
import re

def main():
    root = join(getcwd(), r"node_modules\monaco-editor\esm\vs\basic-languages")
    exts = [".txt"]
    for d in [f for f in listdir(root) if not isfile(join(root, f))]:
        ent = join(root,d);
        for x in listdir(ent):
            if x.endswith("contribution.js"):
                with open(join(ent,x)) as f:
                    txt = f.read()
                    result = re.findall('"\.(?!\.)(?!/).*"',  txt)
                    if len(result):
                        str = re.sub('[\s+]', '', result[0])
                        str = str.replace('"', "")
                        exts.extend(str.split(","))
    print(exts)

main()