import matplotlib.pyplot as plt
import matplotlib
matplotlib.rcParams['font.sans-serif'] = ['SimHei', 'Microsoft YaHei']
matplotlib.rcParams['axes.unicode_minus'] = False

# 浠ｅ竵鍒嗛厤
labels = ['鐢熸€佹寲鐭縗\n40%', '鍥㈤槦\\n20%\\n(4骞磋В閿?', '鎶曡祫鑰匼\n15%\\n(2骞磋В閿?', '鍩洪噾浼歕\n15%', '鍏紑鍙戝敭\\n10%']
sizes = [40, 20, 15, 15, 10]
colors = ['#3498db', '#e74c3c', '#f39c12', '#2ecc71', '#9b59b6']
explode = (0.1, 0, 0, 0, 0)

fig, ax = plt.subplots(figsize=(10, 8))
wedges, texts, autotexts = ax.pie(sizes, explode=explode, labels=labels, colors=colors,
                                    autopct='%1.1f%%', startangle=90, 
                                    textprops={'fontsize': 12, 'weight': 'bold'},
                                    pctdistance=0.85)

# 璁剧疆鐧惧垎姣旀牱寮?
for autotext in autotexts:
    autotext.set_color('white')
    autotext.set_fontweight('bold')
    autotext.set_fontsize(14)

ax.set_title('\$SUPERVM 浠ｅ竵鍒嗛厤\\n鎬讳緵搴旈噺: 1,000,000,000', 
             fontsize=16, fontweight='bold', pad=25)

# 娣诲姞鍥句緥
ax.legend(wedges, labels, title='鍒嗛厤绫诲埆', loc='center left', 
          bbox_to_anchor=(1, 0, 0.5, 1), fontsize=11)

plt.tight_layout()
plt.savefig('visuals/charts/tokenomics.png', dpi=300, bbox_inches='tight')
print('Done: 浠ｅ竵鍒嗛厤鍥惧凡鐢熸垚: visuals/charts/tokenomics.png')
