;;! target = "s390x"
;;! test = "compile"
;;! flags = " -C cranelift-enable-heap-access-spectre-mitigation -O static-memory-maximum-size=0 -O static-memory-guard-size=0 -O dynamic-memory-guard-size=0"

;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
;; !!! GENERATED BY 'make-load-store-tests.sh' DO NOT EDIT !!!
;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

(module
  (memory i32 1)

  (func (export "do_store") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8 offset=0xffff0000)

  (func (export "do_load") (param i32) (result i32)
    local.get 0
    i32.load8_u offset=0xffff0000))

;; wasm[0]::function[0]:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r10, %r15, 0x50(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       lgr     %r10, %r2
;;       llgfr   %r3, %r4
;;       llilf   %r2, 0xffff0001
;;       algfr   %r2, %r4
;;       jgnle   0x3c
;;       lgr     %r14, %r10
;;       lg      %r10, 0x68(%r14)
;;       lghi    %r4, 0
;;       ag      %r3, 0x60(%r14)
;;       llilh   %r11, 0xffff
;;       agr     %r3, %r11
;;       clgr    %r2, %r10
;;       locgrh  %r3, %r4
;;       stc     %r5, 0(%r3)
;;       lmg     %r10, %r15, 0xf0(%r15)
;;       br      %r14
;;
;; wasm[0]::function[1]:
;;       lg      %r1, 8(%r2)
;;       lg      %r1, 0(%r1)
;;       la      %r1, 0xa0(%r1)
;;       clgrtle %r15, %r1
;;       stmg    %r10, %r15, 0x50(%r15)
;;       lgr     %r1, %r15
;;       aghi    %r15, -0xa0
;;       stg     %r1, 0(%r15)
;;       lgr     %r3, %r2
;;       llgfr   %r2, %r4
;;       llilf   %r5, 0xffff0001
;;       algfr   %r5, %r4
;;       jgnle   0xac
;;       lgr     %r14, %r3
;;       lg      %r4, 0x68(%r14)
;;       lghi    %r3, 0
;;       ag      %r2, 0x60(%r14)
;;       llilh   %r10, 0xffff
;;       agr     %r2, %r10
;;       clgr    %r5, %r4
;;       locgrh  %r2, %r3
;;       llc     %r2, 0(%r2)
;;       lmg     %r10, %r15, 0xf0(%r15)
;;       br      %r14