cmake_minimum_required(VERSION 3.16)
project(ducc_full_transform_parity LANGUAGES CXX)

if(NOT DEFINED DUCC_SRC)
  message(FATAL_ERROR "Set DUCC_SRC to the pinned DUCC source checkout")
endif()

find_package(Threads REQUIRED)

set(DUCC_SOURCES
  ${DUCC_SRC}/src/ducc0/fft/fft_inst1.cc
  ${DUCC_SRC}/src/ducc0/fft/fft_inst2.cc
  ${DUCC_SRC}/src/ducc0/nufft/spreadinterp_inst1.cc
  ${DUCC_SRC}/src/ducc0/nufft/spreadinterp_inst2.cc
  ${DUCC_SRC}/src/ducc0/healpix/healpix_base.cc
  ${DUCC_SRC}/src/ducc0/healpix/healpix_tables.cc
  ${DUCC_SRC}/src/ducc0/math/gl_integrator.cc
  ${DUCC_SRC}/src/ducc0/math/pointing.cc
  ${DUCC_SRC}/src/ducc0/math/gridding_kernel.cc
  ${DUCC_SRC}/src/ducc0/math/geom_utils.cc
  ${DUCC_SRC}/src/ducc0/math/wigner3j.cc
  ${DUCC_SRC}/src/ducc0/math/space_filling.cc
  ${DUCC_SRC}/src/ducc0/wgridder/wgridder_inst1.cc
  ${DUCC_SRC}/src/ducc0/wgridder/wgridder_inst2.cc
  ${DUCC_SRC}/src/ducc0/wgridder/wgridder_inst3.cc
  ${DUCC_SRC}/src/ducc0/infra/string_utils.cc
  ${DUCC_SRC}/src/ducc0/infra/threading.cc
  ${DUCC_SRC}/src/ducc0/infra/mav.cc
  ${DUCC_SRC}/src/ducc0/sht/sht.cc)

add_executable(ducc_full_transform_parity ducc_full_transform_parity.cpp ${DUCC_SOURCES})
target_compile_features(ducc_full_transform_parity PRIVATE cxx_std_17)
target_include_directories(ducc_full_transform_parity PRIVATE ${DUCC_SRC}/src)
target_compile_definitions(ducc_full_transform_parity PRIVATE
  DUCC0_NAMESPACE=ducc0 PKGNAME=ducc0 PKGVERSION=0.42.0)
target_compile_options(ducc_full_transform_parity PRIVATE
  -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3)
target_link_libraries(ducc_full_transform_parity PRIVATE Threads::Threads)
