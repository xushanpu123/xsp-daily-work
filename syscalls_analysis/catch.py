import re
import os
import sys
import os.path
import numpy as np
import pandas as pd
import openpyxl as op
with open('./syscalls2.h', 'r') as f:
    syscalls = f.readlines()

syscall_start = None
with open('./unistd2.h', 'r') as f:
    arr = []
    for line in f.readlines():
        m = re.search('#define __.+ ([0-9]+)', line)
        if m:
            syscall_num = f'{m.group(1)}'
            syscall_start = f'SYSCALL({m.group(1)},'
            continue

        m = re.search('__S.+,(.+?)\)',line)
        if m:
            if not syscall_start:
                continue
            syscall = m.group(1).strip()
            # a little fuction name preprocessing
            syscall = re.sub('^compat_', '', syscall)
            syscall = re.sub('^sys', '', syscall)
            # find syscall declaration and print it out
            definitions = [s for s in syscalls if f'{syscall}(' in s]
            if len(definitions) == 0:
                # syscall definition not found!
                syscall_start = None
                continue
            print(f'{syscall_start}"{syscall}","{definitions[0].strip()}")')
            res_array = [syscall_num,syscall,definitions[0].strip()]
            arr.append(res_array)
            syscall_start = None
    df=pd.DataFrame(arr)
	#print(df)
	#df.to_csv(df,"./res.csv")
    wb=op.Workbook()
    ws=wb.active
    i=1
    r=1
    for line in arr:
        for col in range(1,len(line)+1):
            ws.cell(row=r,column=col).value=line[col-1]
        i+=1
        r+=1
    wb.save("./res.xlsx")