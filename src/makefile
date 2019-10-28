# 
# Copyright 2006 Anthony J. Lorelli
# $Id: makefile 281 2006-03-12 15:10:58Z  $
#

all: 
	mcs -out:scheme.exe *.cs
	cp *.scm ../bin/
	mv scheme.exe ../bin/

test: all
	mono ../bin/scheme.exe ../test/test.scm

clean:
	rm -f scheme.exe
	rm -f ../bin/scheme.exe
