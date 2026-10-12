# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

"""Check that the editable installation loads the compiled Rust extension."""

from importlib.machinery import EXTENSION_SUFFIXES
from importlib.metadata import version

import rustsht


def test_native_extension_imports() -> None:
    assert rustsht.__version__ == version("rustsht")
    assert rustsht._rustsht.__name__ == "rustsht._rustsht"
    assert rustsht._rustsht.__file__ is not None
    assert rustsht._rustsht.__file__.endswith(tuple(EXTENSION_SUFFIXES))


def test_typing_marker_is_packaged() -> None:
    from importlib.resources import files

    assert files("rustsht").joinpath("py.typed").is_file()
