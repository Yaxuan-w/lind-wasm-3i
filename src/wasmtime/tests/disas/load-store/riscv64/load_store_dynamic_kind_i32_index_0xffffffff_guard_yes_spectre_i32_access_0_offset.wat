;;! target = "riscv64"
;;! test = "compile"
;;! flags = " -C cranelift-enable-heap-access-spectre-mitigation -O static-memory-maximum-size=0 -O static-memory-guard-size=4294967295 -O dynamic-memory-guard-size=4294967295"

;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
;; !!! GENERATED BY 'make-load-store-tests.sh' DO NOT EDIT !!!
;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

(module
  (memory i32 1)

  (func (export "do_store") (param i32 i32)
    local.get 0
    local.get 1
    i32.store offset=0)

  (func (export "do_load") (param i32) (result i32)
    local.get 0
    i32.load offset=0))

;; wasm[0]::function[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       ld      a4, 0x68(a0)
;;       ld      a1, 0x60(a0)
;;       slli    a5, a2, 0x20
;;       srli    a2, a5, 0x20
;;       sltu    a4, a4, a2
;;       add     a0, a1, a2
;;       neg     a4, a4
;;       not     a1, a4
;;       and     a2, a0, a1
;;       sw      a3, 0(a2)
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
;;       ld      a3, 0x68(a0)
;;       ld      a1, 0x60(a0)
;;       slli    a5, a2, 0x20
;;       srli    a2, a5, 0x20
;;       sltu    a3, a3, a2
;;       add     a0, a1, a2
;;       neg     a4, a3
;;       not     a1, a4
;;       and     a2, a0, a1
;;       lw      a0, 0(a2)
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret