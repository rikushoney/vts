install:
    pip install -r requirements/dev.txt

format:
    ruff check --select I --fix python/vts
    black --quiet python/vts

lock:
    pip-compile -o requirements/tests.txt requirements/tests.in 
    pip-compile -o requirements/check.txt requirements/check.in 
    pip-compile -o requirements/format.txt requirements/format.in 
    pip-compile -o requirements/build.txt requirements/build.in 
    pip-compile -o requirements/dev.txt requirements/dev.in 
