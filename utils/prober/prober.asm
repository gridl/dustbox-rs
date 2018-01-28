    org  0x100        ; .com files always start 256 bytes into the segment

section .data
    %include "data.inc.asm"

section .bss
    ; uninitialized data


section .text
    ; program code
start:
    ; call clear_regs
    ; call clear_mem

    ; -------------------------
    ; run a snippet to analyse:
    ; -------------------------

    push word 0x8000
    pop es
    mov si, 0x100
    mov byte [es:si], 9
    mov dx, 0x03C8
    es outsb    ; reads byte from SI (9) and output it to port DX


    ; save reg states after instruction executes
    mov [_ax], ax
    mov [_bx], bx
    mov [_cx], cx
    mov [_dx], dx
    mov [_sp], sp
    mov [_bp], bp
    mov [_si], si
    mov [_di], di

    mov [_es], es
    mov [_cs], cs
    mov [_ss], ss
    mov [_ds], ds
    mov [_fs], fs
    mov [_gs], gs

    ; read FLAGS 16bit reg
    pushf
    pop ax
    mov [_flags], ax


    call print_regs


    call test_instr

    mov  ah, 0x4c       ; exit to dos
    int  0x21

; tests instructions for correct emulation
test_instr:
    ; TEST1: "idiv r8"
    mov ax, 0x30
    mov bl, 2
    idiv bl
    cmp ax, 0x0018
    je t2
    mov dx, test1fail
    call print_dollar_dx

t2:
    ; TEST2: "idiv r16"
    mov dx, 0x0
    mov ax, 0x8000
    mov bx, 4
    idiv bx
    cmp ax, 0x2000
    je t3
    cmp dx, 0
    je t3
    mov dx, test2fail
    call print_dollar_dx


t3:
    ret

%include "regs.inc.asm"
%include "print.inc.asm"
