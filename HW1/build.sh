pandoc --pdf-engine latexmk \
    -V papersize=a4paper -V fontsize=12pt \
    -V geometry:margin=1in -V mainfont=Times \
    -s README.md -o README.pdf
