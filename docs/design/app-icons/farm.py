"""#225 (q1289): Farm's Bay and Black on a dark background, rendered by Corral's own generator.

Corral's design gate (`docs/design/evidence/issue-462-horse-icons/tools/gen_icons.py` at 060220c3) draws the four
Treatment-A masters with one `BG` per coat. Nothing else changes here: same canonical `horsesvg.py`, crop, pose and
supersampling. Only Bay and Black move from Corral's light grounds onto Farm's two dark ones, the ones Palomino and Grey
already use. Rendering Palomino and Grey this way reproduces Corral's shipped masters byte for byte, which proves
the pipeline matches.

Usage: uv run --no-project --with pillow python docs/design/app-icons/farm.py <corral checkout at 060220c3>
"""

import hashlib
import sys
from pathlib import Path

CORRAL = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(CORRAL / 'docs/design/evidence/issue-462-horse-icons/tools'))
import gen_icons as g  # noqa: E402

OUT = Path(__file__).resolve().parent / 'icons'
GROUND = {'bay': '#1f2a38', 'palomino': '#1f2a38', 'black': '#22302a', 'grey': '#22302a'}
SHIPPED = {'palomino': '7e04da3407296c11f94f929124d4b6c82d83bce70e3a2700c6af906774d863ec',
           'grey': '2d43e4c3a360ddc8d8fade528f227b8808b2bb2baaef0bb816a0373cb738402e'}

prims = {c: g.svgmini.parse_primitives(g.horsesvg.horse_svg(g.identity(c), g.POSE, False, g.FACING, False))
         for c in g.COATS}
window = g.crop_windows(prims['bay'])[0]
for coat in g.COATS:
    out = OUT / f'farm-{coat}.png'
    g.render_window(prims[coat], window, GROUND[coat], out)
    digest = hashlib.sha256(out.read_bytes()).hexdigest()
    if coat in SHIPPED and digest != SHIPPED[coat]:
        sys.exit(f'farm-{coat}.png drifted from Corral\'s shipped master: {digest}')
    print(f'farm-{coat}.png {GROUND[coat]} {digest}')
