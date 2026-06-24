import sys
import os

# Add parent directory to path so tests can import from pii_public_entry_lint
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
