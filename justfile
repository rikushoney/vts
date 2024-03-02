install:
   pip install -r requirements/dev.txt

format:
   ruff check --select I --fix python/vts
   black --quiet python/vts

lock:
   pip-compile requirements/tests.in > requirements/tests.txt
   pip-compile requirements/check.in > requirements/check.txt
   pip-compile requirements/format.in > requirements/format.txt
   pip-compile requirements/build.in > requirements/build.txt
   pip-compile requirements/dev.in > requirements/dev.txt
