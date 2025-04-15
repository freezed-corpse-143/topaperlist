$projectRoot = $PSScriptRoot
$paperDir = Join-Path -Path $projectRoot -ChildPath "Paper"

while ($true) {
    $keywordsInput = Read-Host "Please type keywords (or type 'exit' to quit):"
    
    if ($keywordsInput -eq 'exit') {
        Write-Output "Exiting script..."
        break
    }

    $keywords = $keywordsInput -split ' ' | ForEach-Object { $_.ToLower() }

    Get-ChildItem -Path $paperDir -Directory | ForEach-Object {
        $categoryDir = $_.FullName
        $category = $_.Name
        
        Get-ChildItem -Path $categoryDir -Directory | ForEach-Object {
            $conferenceDir = $_.FullName
            $conferenceName = $_.Name

            Get-ChildItem -Path $conferenceDir -Filter *.txt | ForEach-Object {
                $year = $_.BaseName
                $filePath = $_.FullName

                Get-Content $filePath | ForEach-Object {
                    $title = $_
                    $lowerTitle = $title.ToLower()

                    $containsAllKeywords = ($keywords | ForEach-Object { $lowerTitle -match [regex]::Escape($_) }) -notcontains $false

                    if ($containsAllKeywords) {
                        Write-Output "$category`t$conferenceName`t$year`t$title"
                    }
                }
            }
        }
    }
}
