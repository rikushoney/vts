import nox


@nox.session
def tests(session: nox.Session):
    session.install("maturin", "pytest")
    session.run("maturin", "develop")
    session.run("pytest")


@nox.session
def check(session: nox.Session):
    session.install("mypy", "ruff")
    session.run("ruff", "check", "python/vts")
    session.run("mypy", "python/vts")


@nox.session
def format(session: nox.Session):
    session.install("black", "ruff")
    session.run("ruff", "check", "--select", "I", "python/vts")
    session.run("black", "--check", "python/vts")


@nox.session
def build(session):
    session.install("maturin")
    session.run("maturin", "build")
