;;! target = "riscv64"
;;! test = "compile"
;;! flags = " -C cranelift-enable-heap-access-spectre-mitigation -O static-memory-forced -O static-memory-guard-size=0 -O dynamic-memory-guard-size=0"

;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
;; !!! GENERATED BY 'make-load-store-tests.sh' DO NOT EDIT !!!
;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

(module
  (memory i32 1)

  (func (export "do_store") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8 offset=0)

  (func (export "do_load") (param i32) (result i32)
    local.get 0
    i32.load8_u offset=0))

;; wasm[0]::function[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       ld      a1, 0x60(a0)
;;       slli    a0, a2, 0x20
;;       srli    a2, a0, 0x20
;;       add     a1, a1, a2
;;       sb      a3, 0(a1)
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[1]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       ld      a1, 0x60(a0)
;;       slli    a0, a2, 0x20
;;       srli    a2, a0, 0x20
;;       add     a1, a1, a2
;;       lbu     a0, 0(a1)
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret