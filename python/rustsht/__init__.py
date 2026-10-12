# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

"""Rust spherical harmonic transforms."""

from importlib.metadata import version

from . import _rustsht as _rustsht
from ._rustsht import ShtPlan

__version__ = version("rustsht")

__all__ = ["ShtPlan", "__version__"]
