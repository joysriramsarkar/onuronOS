# hal/Makefile — Compile reference HAL shared libraries
CC ?= gcc
CXX ?= g++
CFLAGS ?= -O2 -fPIC -Iinclude -Wall -Wextra
CXXFLAGS ?= -O2 -fPIC -Iinclude -Wall -Wextra

all: libnilhal_light.so libnilhal_camera.so libnilhal_fp.so

libnilhal_light.so: light/light.c
	$(CC) $(CFLAGS) -shared -o $@ $<

libnilhal_camera.so: camera/camera.cpp
	$(CXX) $(CXXFLAGS) -shared -o $@ $<

libnilhal_fp.so: fingerprint/fingerprint.c
	$(CC) $(CFLAGS) -shared -o $@ $<

clean:
	rm -f *.so
