# 定义项目根目录（脚本所在目录）
$projectRoot = $PSScriptRoot

# 获取paper目录的路径
$paperDir = Join-Path -Path $projectRoot -ChildPath "paper"

# 循环提示用户输入关键词
while ($true) {
    # 提示用户输入关键词
    $keywordsInput = Read-Host "Please type keywords (or type 'exit' to quit):"
    
    # 如果用户输入 'exit'，退出循环
    if ($keywordsInput -eq 'exit') {
        Write-Output "Exiting script..."
        break
    }

    # 将输入的关键词拆分为列表并转换为小写
    $keywords = $keywordsInput -split ' ' | ForEach-Object { $_.ToLower() }

    # 遍历 paper 目录下的所有会议文件夹（包括 A, B, C, D 等）
    Get-ChildItem -Path $paperDir -Directory | ForEach-Object {
        $categoryDir = $_.FullName  # 当前会议水平对应的目录
        $category = $_.Name
        
        Get-ChildItem -Path $categoryDir -Directory | ForEach-Object {
            $conferenceDir = $_.FullName
            $conferenceName = $_.Name

            # 遍历会议文件夹中的所有 .txt 文件
            Get-ChildItem -Path $conferenceDir -Filter *.txt | ForEach-Object {
                $year = $_.BaseName
                $filePath = $_.FullName

                # 读取文件中的每一行（文章标题）
                Get-Content $filePath | ForEach-Object {
                    $title = $_
                    $lowerTitle = $title.ToLower()

                    # 检查标题是否包含所有关键词
                    $containsAllKeywords = ($keywords | ForEach-Object { $lowerTitle -match [regex]::Escape($_) }) -notcontains $false

                    if ($containsAllKeywords) {
                        # 打印符合条件的文章信息，包括会议水平
                        Write-Output "$category`t$conferenceName`t$year`t$title"
                    }
                }
            }
        }
    }
}