import os
import sys

if __name__ == "__main__":
    basedir = os.path.dirname(__file__)
    for path in sorted(os.listdir(basedir)):
        if path.endswith(".py") and path != "__main__.py":
            status = os.system(f"python {basedir}/{path}")
            if status:
                print(f"error: {path} test failed")
                exit(1)
