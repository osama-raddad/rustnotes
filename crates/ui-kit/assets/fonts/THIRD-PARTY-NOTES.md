# Third-party font notes

The faces under `assets/fonts/` are prepared by `tools/prepare_fonts.py`
from the Vynatix design-system font sources and embedded with
`include_bytes!`.

- **Aptos** (Aptos.ttf, Aptos-SemiBold.ttf): Microsoft's typeface. It may
  ship in Vynatix products only; check Microsoft's terms before
  distributing it outside Vynatix's own documents and apps.
- **Fraunces** (Fraunces-Display.ttf, Fraunces-Headline.ttf,
  Fraunces-Metric.ttf, Fraunces-EditorialItalic.ttf): SIL Open Font
  License upstream (underdale.org). The source zip the design system
  hosts carried no licence text; see https://openfontlicense.org for
  the terms.
- **Geist** (Geist-Regular.ttf, Geist-Medium.ttf, Geist-SemiBold.ttf):
  SIL Open Font License upstream (Vercel). No licence text shipped in
  the source download; see https://openfontlicense.org for the terms.
- **Instrument Serif** (InstrumentSerif-Regular.ttf,
  InstrumentSerif-Italic.ttf): SIL Open Font License; the full licence
  text sits next to this file as `InstrumentSerif-OFL.txt`.
- **JetBrains Mono** (JetBrainsMono-Regular.ttf): SIL Open Font License;
  the full licence text sits next to this file as
  `JetBrainsMono-OFL.txt`.

The instanced outputs are derivatives of their OFL sources and keep the
OFL terms: they may be used, embedded and redistributed under the same
licence, and none of them may be sold by itself.
