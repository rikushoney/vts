import nox


@nox.session
def tests(session: nox.Session):
    session.install("-r", "requirements/tests.txt")
    session.run("maturin", "develop")
    session.run("pytest")


@nox.session
def check(session: nox.Session):
    session.install("-r", "requirements/check.txt")
    session.run("ruff", "check", "python/vts")
    session.run("mypy", "python/vts")


@nox.session
def format(session: nox.Session):
    session.install("-r", "requirements/format.txt")
    session.run("ruff", "check", "--select", "I", "python/vts")
    session.run("black", "--check", "python/vts")


@nox.session
def build(session):
    session.install("-r", "requirements/build.txt")
    session.run("maturin", "build")
