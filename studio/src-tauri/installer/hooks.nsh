; The Windows installer's look and texts (bundle.windows.nsis.installerHooks in tauri.conf.json).
; Tauri's NSIS template includes this file after MUI2.nsh and before its first page, so the
; interface settings below take effect, and each page's settings reach the page they name. Plain
; ASCII: NSIS reads a file without a byte-order mark in the system's code page.
;
; The pictures (sidebar.bmp, header.bmp) come from assets/brand's generator; see its README.

; what is being copied, shown as it is, not behind "Show details"
ShowInstDetails show
ShowUninstDetails show

; the mark on the right of every page's header, beside its title
!define MUI_HEADERIMAGE_RIGHT

; the copied files on dark oxide, in the mark's cream, under a smooth bar
!define MUI_INSTFILESPAGE_COLORS "FBE7C6 161010"
!define MUI_INSTFILESPAGE_PROGRESSBAR "smooth"

; the welcome page
!define MUI_WELCOMEPAGE_TITLE "Welcome to photonoxide"
!define MUI_WELCOMEPAGE_TEXT "This installs $(^NameDA) for you: photonics simulation, from a waveguide's modes to whole circuits, heaters and modulators, with the studio that runs your jobs live, replays their records and reproduces published results.$\r$\n$\r$\nIt looks for new releases itself, and offers each one before installing it.$\r$\n$\r$\nClick Next to continue."

; the licence page: permissive licences, nothing to accept to use the program
!define MUI_LICENSEPAGE_TEXT_TOP "photonoxide is free and open source, under the MIT or the Apache 2.0 licence, as you choose."
!define MUI_LICENSEPAGE_TEXT_BOTTOM "Using it asks nothing of you: the licences set the terms for copying and changing it. Click Next to continue."
!define MUI_LICENSEPAGE_BUTTON "$(^NextBtn)"

; the finish page: where the lessons, examples and validation report are
!define MUI_FINISHPAGE_LINK "photonoxide's page: lessons, examples and the validation report"
!define MUI_FINISHPAGE_LINK_LOCATION "https://tachsin.gr/projects/photonoxide"
!define MUI_FINISHPAGE_LINK_COLOR "B7410E"
