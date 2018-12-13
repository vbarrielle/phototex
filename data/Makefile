
R = PHOTOTEX_TOPLEVEL_FILE_NAME

all: pdf

TEX=$(wildcard *.tex)
TOWATCH=$(TEX)

inotify:
	while inotifywait -e delete_self -e modify $(TOWATCH) ; do \
		sleep 1 ; \
		echo "============ at `date` ==========" ; \
		make all ; \
	done

pdf: $(TEX) # title.pdf
	rubber -f --pdf $(R)

clean:
	rubber --clean $(R)

